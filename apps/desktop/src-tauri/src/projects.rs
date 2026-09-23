use crate::{
    core::{AppError, AppState, Result},
    models::Project,
};
use std::path::Path;
use tauri::{Emitter, State};
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InterruptedSession {
    id: String,
    project_id: String,
    created_at: String,
}
#[tauri::command]
pub async fn interrupted_sessions(state: State<'_, AppState>) -> Result<Vec<InterruptedSession>> {
    let rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT id,project_id,created_at FROM agent_sessions WHERE status='interrupted' ORDER BY created_at DESC",
    ).fetch_all(&state.pool).await?;
    Ok(rows
        .into_iter()
        .map(|(id, project_id, created_at)| InterruptedSession {
            id,
            project_id,
            created_at,
        })
        .collect())
}
#[tauri::command]
pub async fn acknowledge_recovery(id: String, state: State<'_, AppState>) -> Result<()> {
    crate::security::valid_id(&id)?;
    sqlx::query("UPDATE agent_sessions SET status='recovered',completed_at=? WHERE id=? AND status='interrupted'")
        .bind(chrono::Utc::now().to_rfc3339()).bind(id).execute(&state.pool).await?;
    Ok(())
}
pub async fn get(state: &AppState, id: &str) -> Result<Project> {
    crate::security::valid_id(id)?;
    let json: Option<String> = sqlx::query_scalar("SELECT payload FROM projects WHERE id=?")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?;
    let p: Project = serde_json::from_str(
        &json.ok_or_else(|| AppError::Invalid("Project was not found".into()))?,
    )?;
    p.validate()?;
    Ok(p)
}
#[tauri::command]
pub async fn list_projects(state: State<'_, AppState>) -> Result<Vec<Project>> {
    let rows: Vec<String> =
        sqlx::query_scalar("SELECT payload FROM projects ORDER BY updated_at DESC")
            .fetch_all(&state.pool)
            .await?;
    let mut result = Vec::new();
    for raw in rows {
        let p: Project = serde_json::from_str(&raw)?;
        p.validate()?;
        result.push(p);
    }
    Ok(result)
}
#[tauri::command]
pub async fn save_project_thumbnail(
    id: String,
    revision_id: String,
    thumbnail: String,
    state: State<'_, AppState>,
) -> Result<Project> {
    crate::security::valid_id(&id)?;
    crate::security::valid_id(&revision_id)?;
    if !thumbnail.starts_with("data:image/jpeg;base64,") || thumbnail.len() > 300_000 {
        return Err(AppError::Invalid("Invalid project thumbnail".into()));
    }
    let _lock = state.writes.lock().await;
    let mut project = get(&state, &id).await?;
    if project.current_revision.as_deref() != Some(&revision_id) {
        return Ok(project);
    }
    project.thumbnail = Some(thumbnail);
    project.thumbnail_revision = Some(revision_id);
    persist(&state, project, Vec::new(), None).await
}

#[tauri::command]
pub async fn delete_project(id: String, state: State<'_, AppState>) -> Result<()> {
    crate::security::valid_id(&id)?;
    let _lock = state.writes.lock().await;
    if state.tasks.lock().await.contains_key(&id) {
        return Err(AppError::Invalid(
            "A project task is already running".into(),
        ));
    }
    let project = get(&state, &id).await?;
    let project_dir = crate::security::guarded(&state.root, Path::new(&id))?;
    let staged = crate::security::guarded(&state.root, Path::new(&format!(".deleting-{id}")))?;
    if staged.exists() {
        return Err(AppError::Invalid(
            "An incomplete project deletion must be resolved first".into(),
        ));
    }
    let moved = project_dir.exists();
    if moved {
        std::fs::rename(&project_dir, &staged)?;
    }
    let deleted = async {
        let mut tx = state.pool.begin().await?;
        for table in ["permissions", "agent_sessions", "activities"] {
            sqlx::query(&format!("DELETE FROM {table} WHERE project_id=?"))
                .bind(&id)
                .execute(&mut *tx)
                .await?;
        }
        sqlx::query("DELETE FROM projects WHERE id=?")
            .bind(&id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok::<(), AppError>(())
    }
    .await;
    if let Err(error) = deleted {
        if moved {
            std::fs::rename(&staged, &project_dir)?;
        }
        return Err(error);
    }
    state
        .grants
        .lock()
        .await
        .retain(|_, grant| grant.project_id != id);
    if moved {
        std::fs::remove_dir_all(&staged)?;
    }
    let remaining: Vec<String> = sqlx::query_scalar("SELECT payload FROM projects")
        .fetch_all(&state.pool)
        .await?;
    let used: std::collections::HashSet<String> = remaining
        .iter()
        .filter_map(|raw| serde_json::from_str::<Project>(raw).ok())
        .flat_map(|other| other.files.into_iter().filter_map(|file| file.sha256))
        .collect();
    for hash in project.files.iter().filter_map(|file| file.sha256.as_ref()) {
        if used.contains(hash) {
            continue;
        }
        let blob = crate::security::guarded(&state.root, Path::new(&format!(".blobs/{hash}")))?;
        match std::fs::remove_file(blob) {
            Ok(()) => (),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
            Err(error) => {
                tracing::warn!(%error, %hash, "failed to remove unreferenced project blob");
            }
        }
    }
    Ok(())
}
pub fn validate_history(old: &Project, new: &Project) -> Result<bool> {
    if old.files.iter().any(|file| !new.files.contains(file)) {
        return Err(AppError::Invalid(
            "Saved source files are immutable. Import changes with a new filename.".into(),
        ));
    }
    if new.revisions.len() < old.revisions.len() || !new.revisions.starts_with(&old.revisions) {
        return Err(AppError::Invalid(
            "Saved revisions are immutable. Restore by appending a new revision.".into(),
        ));
    }
    if new.revisions.len() > old.revisions.len() + 1 {
        return Err(AppError::Invalid(
            "Only one new revision can be saved at a time".into(),
        ));
    }
    if new.revisions.len() > old.revisions.len() {
        let last = &new.revisions[new.revisions.len() - 1];
        if let Some(current) = old
            .revisions
            .iter()
            .find(|r| Some(&r.id) == old.current_revision.as_ref())
        {
            if last.source.is_none()
                && current.source.is_none()
                && last.parameters.same_geometry(&current.parameters)
            {
                return Err(AppError::Invalid(
                    "Model geometry is unchanged; no new revision is needed".into(),
                ));
            }
        }
        if last.parent != old.current_revision || new.current_revision.as_ref() != Some(&last.id) {
            return Err(AppError::Invalid(
                "Revision must extend the current model".into(),
            ));
        }
    } else if new.current_revision != old.current_revision {
        return Err(AppError::Invalid(
            "Changing the revision pointer is not an undo operation".into(),
        ));
    }
    Ok(new.revisions != old.revisions || new.files != old.files)
}
#[tauri::command]
pub async fn save_project(
    mut project: Project,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<Project> {
    project.validate()?;
    let mut pending = crate::artifacts::normalize(&mut project)?;
    let _lock = state.writes.lock().await;
    let old: Option<String> = sqlx::query_scalar("SELECT payload FROM projects WHERE id=?")
        .bind(&project.id)
        .fetch_optional(&state.pool)
        .await?;
    if let Some(raw) = old {
        let mut old: Project = serde_json::from_str(&raw)?;
        pending.extend(crate::artifacts::normalize(&mut old)?);
        if validate_history(&old, &project)? {
            if state.tasks.lock().await.contains_key(&project.id) {
                return Err(AppError::Invalid(
                    "A modifying task is already active".into(),
                ));
            }
            crate::permissions::consume(&state, &project.id, "modify_project").await?;
        }
    }
    persist(&state, project, pending, Some(&app)).await
}
pub async fn persist(
    state: &AppState,
    project: Project,
    pending: Vec<(String, Vec<u8>)>,
    app: Option<&tauri::AppHandle>,
) -> Result<Project> {
    project.validate()?;
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
    // SQLite is authoritative. JSON is a recoverable sidecar, never used to overwrite DB state.
    sqlx::query("INSERT INTO projects(id,name,payload,updated_at) VALUES(?,?,?,?) ON CONFLICT(id) DO UPDATE SET name=excluded.name,payload=excluded.payload,updated_at=excluded.updated_at").bind(&project.id).bind(&project.name).bind(&raw).bind(&project.updated_at).execute(&state.pool).await?;
    let json = crate::security::guarded(&root, Path::new("project.json"))?;
    let temp = crate::security::guarded(&root, Path::new("metadata/project.json.tmp"))?;
    if let Err(e) = std::fs::write(&temp, &raw).and_then(|_| std::fs::rename(&temp, &json)) {
        tracing::warn!(%e,"Project JSON mirror failed; SQLite copy is committed");
    }
    if let Some(current) = project
        .revisions
        .iter()
        .find(|r| Some(&r.id) == project.current_revision.as_ref())
    {
        if let Some(program) = &current.program {
            let file = crate::security::guarded(&root, Path::new("workspace/model.py"))?;
            if let Err(error) = std::fs::write(file, program) {
                tracing::warn!(%error,"Could not mirror current CAD source");
            }
        }
        let file = crate::security::guarded(&root, Path::new("workspace/model.parameters.json"))?;
        if let Err(error) = std::fs::write(file, serde_json::to_vec_pretty(&current.parameters)?) {
            tracing::warn!(%error,"Could not mirror current parameters");
        }
    }
    if let Some(app) = app {
        let _ = app.emit("project://revision-created", &project.id);
    }
    Ok(project)
}
