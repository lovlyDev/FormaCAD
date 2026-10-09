use super::{
    bounded_read, publish, read_folder, read_generation, Manifest, ProjectLock, StagedSnapshot,
};
use crate::{
    core::{AppError, AppState, Result},
    models::Project,
};
use std::path::Path;

/// Finish an interrupted DB->folder mirror only from the committed, validated DB payload.
/// A corrupt existing manifest is preserved and reported; it is never silently repaired.
pub async fn reconcile(state: &AppState, mut project: Project) -> Result<Project> {
    let root = crate::security::guarded(&state.root, Path::new(&project.id))?;
    let pending: Option<(Option<String>, String, String)> = sqlx::query_as("SELECT base_manifest_sha256,target_manifest,project_sha256 FROM project_storage_commits WHERE project_id=?").bind(&project.id).fetch_optional(&state.pool).await?;
    if pending.is_none() && (!root.exists() || !root.join("manifest.json").exists()) {
        return Ok(project);
    }
    if project.files.iter().any(|file| file.data.is_some()) {
        crate::artifacts::normalize(&mut project)?;
    }
    if let Some((base, target, expected_project)) = pending {
        let manifest: Manifest = serde_json::from_str(&target)?;
        let current = if root.join("manifest.json").exists() {
            Some(bounded_read(
                &root,
                Path::new("manifest.json"),
                2 * 1024 * 1024,
            )?)
        } else {
            None
        };
        let already_published = current.as_ref().is_some_and(|bytes| {
            serde_json::from_slice::<Manifest>(bytes)
                .ok()
                .is_some_and(|value| {
                    serde_json::to_vec(&value).ok() == serde_json::to_vec(&manifest).ok()
                })
        });
        if crate::artifacts::digest(&serde_json::to_vec(&project)?) != expected_project
            || (!already_published
                && current
                    .as_ref()
                    .map(|bytes| crate::artifacts::digest(bytes))
                    != base)
        {
            return Err(AppError::Invalid(
                "Project storage recovery conflicts with externally changed data".into(),
            ));
        }
        let recovered = read_generation(&root, &manifest)?;
        if serde_json::to_vec(&recovered)? != serde_json::to_vec(&project)? {
            return Err(AppError::Invalid(
                "Project storage recovery payload mismatch".into(),
            ));
        }
        let _access = match crate::project_access::ensure_write(state, &project.id) {
            Ok(guard) => guard,
            Err(AppError::Invalid(message)) if message == "PROJECT_READ_ONLY" => {
                // The durable generation can be viewed while its owner retries publication.
                // A second process must never publish or clear the owner's journal.
                return Ok(recovered);
            }
            Err(error) => return Err(error),
        };
        let _lock = ProjectLock::acquire(&root)?;
        if !already_published {
            publish(&root, StagedSnapshot { manifest })?;
        }
        sqlx::query("DELETE FROM project_storage_commits WHERE project_id=?")
            .bind(&project.id)
            .execute(&state.pool)
            .await?;
    }
    if !root.exists() || !root.join("manifest.json").exists() {
        return Ok(project);
    }
    let disk = read_folder(&root)?;
    if serde_json::to_vec(&disk)? != serde_json::to_vec(&project)? {
        return Err(AppError::Invalid(
            "Project storage differs from its application index; reopen the folder explicitly"
                .into(),
        ));
    }
    Ok(disk)
}

pub async fn record_pending(
    connection: &mut sqlx::SqliteConnection,
    root: &Path,
    project: &Project,
    staged: &StagedSnapshot,
) -> Result<()> {
    let base = if root.join("manifest.json").exists() {
        Some(crate::artifacts::digest(&bounded_read(
            root,
            Path::new("manifest.json"),
            2 * 1024 * 1024,
        )?))
    } else {
        None
    };
    sqlx::query("INSERT INTO project_storage_commits(project_id,base_manifest_sha256,target_manifest,project_sha256) VALUES(?,?,?,?) ON CONFLICT(project_id) DO UPDATE SET base_manifest_sha256=excluded.base_manifest_sha256,target_manifest=excluded.target_manifest,project_sha256=excluded.project_sha256")
        .bind(&project.id).bind(base).bind(serde_json::to_string(&staged.manifest)?).bind(crate::artifacts::digest(&serde_json::to_vec(project)?)).execute(connection).await?;
    Ok(())
}
