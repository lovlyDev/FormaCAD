//! Read history controls from one committed project/history snapshot.
use super::HistoryStatus;
use crate::{
    core::{AppState, Result},
    project_repository::{self as repository, HistoryStateSnapshot},
};

pub async fn status_for(state: &AppState, project_id: &str) -> Result<HistoryStatus> {
    let snapshot = repository::capture_history_state(state, project_id).await?;
    from_snapshot(state, &snapshot).await
}

async fn from_snapshot(state: &AppState, snapshot: &HistoryStateSnapshot) -> Result<HistoryStatus> {
    let history = &snapshot.history_state;
    let status = HistoryStatus {
        can_undo: history.can_undo(),
        can_redo: history.can_redo(),
        expected_revision: snapshot.project.current_revision.clone(),
    };
    repository::recheck_history_state(state, snapshot).await?;
    Ok(status)
}

#[cfg(test)]
#[path = "status/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "status/session_tests.rs"]
mod session_tests;
