//! Pure committed snapshots, without recovery publication or index writes.
mod folder;
mod sql;
#[cfg(test)]
mod tests;
use crate::{
    core::{AppError, AppState, Result},
    models::Project,
};

pub(super) fn changed() -> AppError {
    AppError::Invalid("PROJECT_SNAPSHOT_CHANGED".into())
}
pub struct CommittedSnapshot {
    pub project: Project,
    pub history: Option<Vec<u8>>,
    raw: sql::RawSnapshot,
    folder: Option<String>,
    project_bytes: Vec<u8>,
}
pub async fn capture_project(state: &AppState, id: &str) -> Result<CommittedSnapshot> {
    capture(state, id, false).await
}
pub async fn capture_with_history(state: &AppState, id: &str) -> Result<CommittedSnapshot> {
    capture(state, id, true).await
}
async fn capture(state: &AppState, id: &str, history: bool) -> Result<CommittedSnapshot> {
    crate::security::valid_id(id)?;
    let raw = sql::read(&state.pool, id, history).await?;
    if raw.journal.is_some() {
        return Err(AppError::Invalid(
            "PROJECT_SNAPSHOT_RECOVERY_REQUIRED".into(),
        ));
    }
    let mut project: Project = serde_json::from_str(&raw.project)?;
    project.validate()?;
    if project.id != id {
        return Err(changed());
    }
    let folder = folder::check(state, &mut project, raw.history.as_deref())?;
    let latest = sql::read(&state.pool, id, history)
        .await
        .map_err(missing_to_changed)?;
    if latest != raw {
        return Err(changed());
    }
    let project_bytes = serde_json::to_vec(&project)?;
    Ok(CommittedSnapshot {
        project,
        history: raw.history.clone(),
        raw,
        folder,
        project_bytes,
    })
}
fn missing_to_changed(error: AppError) -> AppError {
    match error {
        AppError::Invalid(ref code) if code == "PROJECT_SNAPSHOT_MISSING" => changed(),
        other => other,
    }
}
/// Revalidate before using captured bytes; this does not lock a later publication.
pub async fn recheck(state: &AppState, snapshot: &CommittedSnapshot) -> Result<()> {
    if serde_json::to_vec(&snapshot.project)? != snapshot.project_bytes
        || snapshot.history != snapshot.raw.history
    {
        return Err(changed());
    }
    let latest = capture(state, &snapshot.project.id, snapshot.history.is_some())
        .await
        .map_err(missing_to_changed)?;
    if latest.raw != snapshot.raw
        || latest.folder != snapshot.folder
        || latest.project_bytes != snapshot.project_bytes
    {
        return Err(changed());
    }
    Ok(())
}
