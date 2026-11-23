//! Exact staged model preview. No grants consumed, revisions committed or artifacts attached.
use crate::core::AppError;
use crate::core::{AppState, Result};
use std::sync::atomic::Ordering;
use tauri::State;
#[cfg(feature = "native-worker-client")]
pub mod build;
#[cfg(feature = "native-worker-client")]
mod host;
#[cfg(feature = "native-worker-client")]
mod imports;
#[cfg(feature = "native-worker-client")]
mod packet;
#[cfg(feature = "native-worker-client")]
mod workspace;
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn preview_model(
    project_id: String,
    program: String,
    expected_revision: Option<String>,
    source_id: Option<String>,
    expected_commit_id: Option<String>,
    window: tauri::WebviewWindow,
    sources: State<'_, crate::project_repository::window_sources::WindowSources>,
    state: State<'_, AppState>,
) -> Result<tauri::ipc::Response> {
    if source_id.is_some() != expected_commit_id.is_some() {
        return Err(AppError::Invalid("PROJECT_SNAPSHOT_CHANGED".into()));
    }
    #[cfg(not(feature = "native-worker-client"))]
    {
        let _ = (
            project_id,
            program,
            expected_revision,
            state,
            window,
            sources,
            expected_commit_id,
        );
        if source_id.is_some() {
            return Err(AppError::Invalid("PROJECT_FOLDER_UNSUPPORTED".into()));
        }
        Err(AppError::Invalid(
            "Model preview requires the native OpenCascade build".into(),
        ))
    }
    #[cfg(feature = "native-worker-client")]
    {
        if let (Some(key), Some(commit)) = (source_id, expected_commit_id) {
            let read = sources
                .capture_read(window.label(), &key, &project_id, &commit)
                .await?;
            return host::preview_bound(&state, read, &program, expected_revision)
                .await
                .map(tauri::ipc::Response::new);
        }
        host::preview(&state, &project_id, &program, expected_revision)
            .await
            .map(tauri::ipc::Response::new)
    }
}
#[tauri::command]
pub async fn cancel_model_preview(
    project_id: String,
    source_id: Option<String>,
    expected_commit_id: Option<String>,
    window: tauri::WebviewWindow,
    sources: State<'_, crate::project_repository::window_sources::WindowSources>,
    state: State<'_, AppState>,
) -> Result<()> {
    if source_id.is_some() != expected_commit_id.is_some() {
        return Err(AppError::Invalid("PROJECT_SNAPSHOT_CHANGED".into()));
    }
    let scope = match (source_id, expected_commit_id) {
        (Some(key), Some(commit)) => sources
            .capture_read(window.label(), &key, &project_id, &commit)
            .await?
            .task_scope_id()
            .to_owned(),
        (None, None) => project_id.clone(),
        _ => return Err(AppError::Invalid("PROJECT_SNAPSHOT_CHANGED".into())),
    };
    crate::security::valid_id(&project_id)?;
    if let Some(cancel) = state.tasks.lock().await.get(&format!("preview:{scope}")) {
        cancel.store(true, Ordering::Relaxed);
    }
    Ok(())
}
