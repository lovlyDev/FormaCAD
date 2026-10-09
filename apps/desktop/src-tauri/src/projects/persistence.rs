use crate::{
    core::{AppError, AppState, Result},
    models::Project,
};
use std::path::Path;
use tauri::Emitter;

pub async fn persist(
    state: &AppState,
    project: Project,
    pending: Vec<(String, Vec<u8>)>,
    app: Option<&tauri::AppHandle>,
) -> Result<Project> {
    persist_transaction(state, project, pending, app, None, None).await
}

pub async fn persist_with_history(
    state: &AppState,
    project: Project,
    pending: Vec<(String, Vec<u8>)>,
    app: Option<&tauri::AppHandle>,
    history: crate::project_history::PreparedHistory,
) -> Result<Project> {
    persist_transaction(state, project, pending, app, Some(history), None).await
}

pub async fn persist_imported_project(
    state: &AppState,
    project: Project,
    pending: Vec<(String, Vec<u8>)>,
    app: Option<&tauri::AppHandle>,
    history: Option<Vec<u8>>,
) -> Result<Project> {
    persist_transaction(state, project, pending, app, None, history).await
}

async fn persist_transaction(
    state: &AppState,
    project: Project,
    pending: Vec<(String, Vec<u8>)>,
    app: Option<&tauri::AppHandle>,
    history: Option<crate::project_history::PreparedHistory>,
    imported_history: Option<Vec<u8>>,
) -> Result<Project> {
    persist_transaction_inner(
        state,
        project,
        pending,
        app,
        history,
        imported_history,
        PostCommitHook::default(),
    )
    .await
}

#[derive(Default)]
struct PostCommitHook<'a> {
    #[cfg(test)]
    callback: Option<&'a (dyn Fn(&Path) + Send + Sync)>,
    #[cfg(test)]
    before_mirrors: Option<&'a (dyn Fn(&Path) + Send + Sync)>,
    _lifetime: std::marker::PhantomData<&'a ()>,
}

// Private per-request seam for deterministic post-SQL filesystem fault tests.
// No IPC field, global hook, permission bypass, or caller-owned output path.
async fn persist_transaction_inner(
    state: &AppState,
    project: Project,
    mut pending: Vec<(String, Vec<u8>)>,
    app: Option<&tauri::AppHandle>,
    history: Option<crate::project_history::PreparedHistory>,
    imported_history: Option<Vec<u8>>,
    post_commit: PostCommitHook<'_>,
) -> Result<Project> {
    project.validate()?;
    let _access = crate::project_access::ensure_write(state, &project.id)?;
    let old_raw: Option<String> = sqlx::query_scalar("SELECT payload FROM projects WHERE id=?")
        .bind(&project.id)
        .fetch_optional(&state.pool)
        .await?;
    let mut old = old_raw
        .as_deref()
        .map(serde_json::from_str::<Project>)
        .transpose()?;
    if let Some(previous) = old.clone() {
        crate::project_storage_v2::reconcile(state, previous).await?;
    }
    if let Some(previous) = &mut old {
        pending.extend(crate::artifacts::normalize(previous)?);
    }
    let root = crate::security::guarded(&state.root, Path::new(&project.id))?;
    std::fs::create_dir_all(&root)?;
    for folder in [
        "workspace",
        "output",
        "cache",
        "revisions",
        "attachments",
        "metadata",
    ] {
        let dir = crate::security::guarded(&root, Path::new(folder))?;
        std::fs::create_dir_all(dir)?;
    }
    let _storage_lock = crate::project_storage_v2::ProjectLock::acquire(&root)?;
    let locked_raw: Option<String> = sqlx::query_scalar("SELECT payload FROM projects WHERE id=?")
        .bind(&project.id)
        .fetch_optional(&state.pool)
        .await?;
    if locked_raw != old_raw {
        return Err(AppError::Invalid(
            "Project changed; retry the CAD edit".into(),
        ));
    }
    if !root.join("manifest.json").exists() {
        if let Some(mut previous) = old.clone() {
            crate::artifacts::normalize(&mut previous)?;
            crate::artifacts::materialize(state, &previous, &pending)?;
            let baseline = crate::project_storage_v2::stage(state, &root, &previous)?;
            crate::project_storage_v2::publish(&root, baseline)?;
        }
    }
    crate::artifacts::materialize(state, &project, &pending)?;
    for revision in &project.revisions {
        let bytes = serde_json::to_vec_pretty(revision)?;
        crate::artifacts::immutable_write(
            &root,
            Path::new(&format!("revisions/{}.json", revision.id)),
            &bytes,
        )?;
    }
    let raw = serde_json::to_string(&project)?;
    // Stage and validate immutable self-contained files before committing the SQL journal/index.
    let mut staged = crate::project_storage_v2::stage(state, &root, &project)?;
    let mut transaction = state.pool.begin().await?;
    let transaction_raw: Option<String> =
        sqlx::query_scalar("SELECT payload FROM projects WHERE id=?")
            .bind(&project.id)
            .fetch_optional(&mut *transaction)
            .await?;
    if transaction_raw != old_raw {
        return Err(AppError::Invalid(
            "Project changed; retry the CAD edit".into(),
        ));
    }
    sqlx::query("INSERT INTO projects(id,name,payload,updated_at) VALUES(?,?,?,?) ON CONFLICT(id) DO UPDATE SET name=excluded.name,payload=excluded.payload,updated_at=excluded.updated_at").bind(&project.id).bind(&project.name).bind(&raw).bind(&project.updated_at).execute(&mut *transaction).await?;
    if let Some(bytes) = imported_history {
        crate::project_history::import_history(&mut transaction, &project, &bytes).await?;
    } else {
        crate::project_history::record_commit(
            &mut transaction,
            old.as_ref(),
            &project,
            history.as_ref(),
        )
        .await?;
    }
    let history_bytes = crate::project_history::export_history(&mut transaction, &project).await?;
    crate::project_storage_v2::stage_history(&root, &mut staged, &history_bytes)?;
    crate::project_storage_v2::record_pending(&mut transaction, &root, &project, &staged).await?;
    transaction.commit().await?;
    #[cfg(test)]
    if let Some(hook) = post_commit.callback {
        hook(&root);
    }
    #[cfg(not(test))]
    let _ = post_commit;
    let publication = crate::security::guarded(&state.root, Path::new(&project.id))
        .and_then(|managed_root| crate::project_storage_v2::publish(&managed_root, staged));
    match publication {
        Ok(()) => {
            if let Err(error) =
                sqlx::query("DELETE FROM project_storage_commits WHERE project_id=?")
                    .bind(&project.id)
                    .execute(&state.pool)
                    .await
            {
                tracing::warn!(%error, "Committed folder is valid; pending journal cleanup will retry on open");
            }
        }
        Err(error) => {
            tracing::warn!(%error, "SQLite commit is durable; folder publication will recover from its journal on open")
        }
    }
    #[cfg(test)]
    if let Some(hook) = post_commit.before_mirrors {
        hook(&root);
    }
    super::mirrors::update(&state.root, &project, &raw);
    if super::events::revision_changed(old.as_ref(), &project) {
        if let Some(app) = app {
            let _ = app.emit("project://revision-created", &project.id);
        }
    }
    Ok(project)
}

/// Explicit folder import, never silently replace an indexed project with the same identity.
/// The source folder is read-only; save_copy changes only project identity, retaining revision IDs.
pub async fn import_folder(
    state: &AppState,
    path: &Path,
    save_copy: bool,
    app: Option<&tauri::AppHandle>,
) -> Result<Project> {
    import_folder_checked(state, path, save_copy, None, app).await
}

pub async fn import_folder_checked(
    state: &AppState,
    path: &Path,
    save_copy: bool,
    expected_manifest_sha256: Option<&str>,
    app: Option<&tauri::AppHandle>,
) -> Result<Project> {
    let (mut project, history) =
        crate::project_storage_v2::hydrate_checked(path, expected_manifest_sha256)?;
    let _write = state.writes.lock().await;
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM projects WHERE id=?)")
        .bind(&project.id)
        .fetch_one(&state.pool)
        .await?;
    if exists && !save_copy {
        return Err(AppError::Invalid(
            "Project identity already exists; open the existing project or save a copy".into(),
        ));
    }
    if save_copy {
        project.id = uuid::Uuid::new_v4().to_string();
    }
    let pending = crate::artifacts::normalize(&mut project)?;
    project.validate()?;
    persist_transaction(state, project, pending, app, None, history).await
}

#[cfg(test)]
#[path = "persistence_tests.rs"]
mod post_commit_tests;
