//! Pure committed snapshots, without recovery publication or index writes.
mod cursor;
mod cursor_sql;
mod folder;
mod sql;
pub use cursor::{capture_history_state, recheck_history_state, HistoryStateSnapshot};
#[cfg(test)]
pub(crate) mod read_test_fixture;
#[cfg(test)]
mod session_tests;
#[cfg(test)]
mod tests;
use crate::{
    core::{AppError, AppState, Result},
    models::Project,
    project_repository::session::ReadSession,
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
    session: ReadSession,
}
pub async fn capture_project(state: &AppState, id: &str) -> Result<CommittedSnapshot> {
    capture(state, id, false).await
}
pub async fn capture_with_history(state: &AppState, id: &str) -> Result<CommittedSnapshot> {
    capture(state, id, true).await
}
async fn capture(state: &AppState, id: &str, history: bool) -> Result<CommittedSnapshot> {
    crate::security::valid_id(id)?;
    let session = ReadSession::capture(state, id)?;
    let raw = sql::read(&state.pool, id, history).await?;
    session.recheck(state, id)?;
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
    session.recheck(state, id)?;
    if latest != raw {
        return Err(changed());
    }
    let project_bytes = serde_json::to_vec(&project)?;
    session.recheck(state, id)?;
    Ok(CommittedSnapshot {
        project,
        history: raw.history.clone(),
        raw,
        folder,
        project_bytes,
        session,
    })
}
fn missing_to_changed(error: AppError) -> AppError {
    match error {
        AppError::Invalid(ref code) if code == "PROJECT_SNAPSHOT_MISSING" => changed(),
        other => other,
    }
}
/// Validate only the ORIGINAL opening/location and captured payload integrity.
/// This does not compare the current saved payload, cursor or manifest and is
/// NOT a replacement for full `recheck`. Geometry-specific consumers must also
/// recapture and validate their original revision, artifacts and result seals.
/// Metadata-only saved changes can retain this lifecycle while close/reopen or
/// source replacement cannot. This grants no writer lease and performs no SQL.
pub fn recheck_lifecycle(state: &AppState, snapshot: &CommittedSnapshot) -> Result<()> {
    snapshot.session.recheck(state, &snapshot.project.id)?;
    if serde_json::to_vec(&snapshot.project)? != snapshot.project_bytes
        || snapshot.history != snapshot.raw.history
    {
        return Err(changed());
    }
    Ok(())
}

/// Revalidate before using captured bytes; this does not lock a later publication.
pub async fn recheck(state: &AppState, snapshot: &CommittedSnapshot) -> Result<()> {
    recheck_lifecycle(state, snapshot)?;
    let latest = capture(state, &snapshot.project.id, snapshot.history.is_some())
        .await
        .map_err(missing_to_changed)?;
    recheck_lifecycle(state, snapshot)?;
    if latest.raw != snapshot.raw
        || latest.folder != snapshot.folder
        || latest.project_bytes != snapshot.project_bytes
    {
        return Err(changed());
    }
    Ok(())
}
