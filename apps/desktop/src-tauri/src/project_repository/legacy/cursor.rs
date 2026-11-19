//! Pure typed history cursor snapshots; portable event archives remain separate.
use super::{changed, cursor_sql, folder, missing_to_changed};
use crate::{
    core::{AppError, AppState, Result},
    models::Project,
    project_history::HistoryState,
    project_repository::session::ReadSession,
};

pub struct HistoryStateSnapshot {
    pub project: Project,
    pub history_state: HistoryState,
    raw: cursor_sql::RawCursor,
    folder: Option<String>,
    project_bytes: Vec<u8>,
    state_bytes: Vec<u8>,
    session: ReadSession,
}

impl HistoryStateSnapshot {
    pub(super) fn matches_raw(&self, raw: &cursor_sql::RawCursor) -> bool {
        self.raw == *raw
    }
}

pub async fn capture_history_state(state: &AppState, id: &str) -> Result<HistoryStateSnapshot> {
    crate::security::valid_id(id)?;
    let session = ReadSession::capture(state, id)?;
    let raw = cursor_sql::read(&state.pool, id).await?;
    session.recheck(state, id)?;
    if raw.project.journal.is_some() {
        return Err(AppError::Invalid(
            "PROJECT_SNAPSHOT_RECOVERY_REQUIRED".into(),
        ));
    }
    let mut project: Project = serde_json::from_str(&raw.project.project)?;
    project.validate()?;
    if project.id != id {
        return Err(changed());
    }
    let history_state = crate::project_history::state_from_json(&project, raw.state.as_deref())?;
    let folder = folder::check(state, &mut project, None)?;
    let latest = cursor_sql::read(&state.pool, id)
        .await
        .map_err(missing_to_changed)?;
    session.recheck(state, id)?;
    if latest != raw {
        return Err(changed());
    }
    let project_bytes = serde_json::to_vec(&project)?;
    let state_bytes = serde_json::to_vec(&history_state)?;
    session.recheck(state, id)?;
    Ok(HistoryStateSnapshot {
        project,
        history_state,
        raw,
        folder,
        project_bytes,
        state_bytes,
        session,
    })
}

pub async fn recheck_history_state(
    state: &AppState,
    snapshot: &HistoryStateSnapshot,
) -> Result<()> {
    snapshot.session.recheck(state, &snapshot.project.id)?;
    if serde_json::to_vec(&snapshot.project)? != snapshot.project_bytes
        || serde_json::to_vec(&snapshot.history_state)? != snapshot.state_bytes
    {
        return Err(changed());
    }
    let latest = capture_history_state(state, &snapshot.project.id)
        .await
        .map_err(missing_to_changed)?;
    if latest.raw != snapshot.raw
        || latest.folder != snapshot.folder
        || latest.project_bytes != snapshot.project_bytes
        || latest.state_bytes != snapshot.state_bytes
    {
        return Err(changed());
    }
    snapshot.session.recheck(state, &snapshot.project.id)?;
    Ok(())
}
