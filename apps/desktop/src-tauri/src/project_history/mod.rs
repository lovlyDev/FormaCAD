//! Durable model transaction history. Revisions and artifacts remain immutable;
//! only a logical cursor changes, atomically with the saved project payload.
pub mod activity;
mod artifacts;
pub(crate) mod commit;
pub(crate) mod model_context;
mod persistence;
mod portable;
mod read_state;
mod state;
mod status;
pub(crate) use commit::prepare_commit;
pub(crate) use read_state::from_context_json as state_from_context_json;
pub(crate) use read_state::from_json as state_from_json;
pub use status::status_for;

pub use persistence::{load_state, record_commit};
pub use portable::{export_history, import_history, validate_archive};
pub use state::{prepare, Direction, HistoryState, PreparedHistory};
pub(crate) use state::{prepare_amend, AmendCapture};

use crate::{
    core::{AppError, AppState, Result},
    models::Project,
    project_repository::window_sources::{
        write_ipc::{self, Command as FolderCommand, FolderWriteRequest, WriteReply},
        WindowSources,
    },
};
use serde::Serialize;
use tauri::State;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryStatus {
    pub can_undo: bool,
    pub can_redo: bool,
    pub expected_revision: Option<String>,
}

#[tauri::command]
pub async fn history_status(
    project_id: String,
    source_id: Option<String>,
    expected_commit_id: Option<String>,
    window: tauri::WebviewWindow,
    sources: State<'_, crate::project_repository::window_sources::WindowSources>,
    state: State<'_, AppState>,
) -> Result<HistoryStatus> {
    match (source_id, expected_commit_id) {
        (None, None) => status_for(&state, &project_id).await,
        (Some(source_id), Some(commit)) => {
            sources
                .folder_history_status(window.label(), &source_id, &project_id, &commit)
                .await
        }
        _ => Err(AppError::Invalid("PROJECT_SNAPSHOT_CHANGED".into())),
    }
}

pub async fn apply_history(
    state: &AppState,
    project_id: &str,
    expected_revision: Option<&str>,
    direction: Direction,
    app: Option<&tauri::AppHandle>,
) -> Result<Project> {
    crate::security::valid_id(project_id)?;
    if let Some(id) = expected_revision {
        crate::security::valid_id(id)?;
    }
    let _lock = state.writes.lock().await;
    if state.tasks.lock().await.contains_key(project_id) {
        return Err(AppError::Invalid(
            "A project task is already running".into(),
        ));
    }
    let current = crate::projects::get(state, project_id).await?;
    let history = load_state(&state.pool, &current).await?;
    let (project, intent) = prepare(
        &current,
        &history,
        direction,
        expected_revision,
        &uuid::Uuid::new_v4().to_string(),
        &chrono::Utc::now().to_rfc3339(),
    )?;
    // Read verified immutable artifacts before consuming approval or committing.
    artifacts::validate_snapshot(state, &project)?;
    crate::permissions::consume(state, project_id, "modify_project").await?;
    crate::projects::persist_with_history(state, project, Vec::new(), app, intent).await
}

#[tauri::command]
pub async fn undo_model(
    project_id: String,
    expected_revision: Option<String>,
    folder_write: Option<FolderWriteRequest>,
    window: tauri::WebviewWindow,
    sources: State<'_, WindowSources>,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<WriteReply> {
    if let Some(request) = folder_write {
        return write_ipc::execute(
            &sources,
            &state,
            window.label(),
            &project_id,
            request,
            FolderCommand::History(Direction::Undo),
            expected_revision.is_some(),
        )
        .await;
    }
    apply_history(
        &state,
        &project_id,
        expected_revision.as_deref(),
        Direction::Undo,
        Some(&app),
    )
    .await
    .map(WriteReply::from)
}

#[tauri::command]
pub async fn redo_model(
    project_id: String,
    expected_revision: Option<String>,
    folder_write: Option<FolderWriteRequest>,
    window: tauri::WebviewWindow,
    sources: State<'_, WindowSources>,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<WriteReply> {
    if let Some(request) = folder_write {
        return write_ipc::execute(
            &sources,
            &state,
            window.label(),
            &project_id,
            request,
            FolderCommand::History(Direction::Redo),
            expected_revision.is_some(),
        )
        .await;
    }
    apply_history(
        &state,
        &project_id,
        expected_revision.as_deref(),
        Direction::Redo,
        Some(&app),
    )
    .await
    .map(WriteReply::from)
}
