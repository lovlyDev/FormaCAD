//! Exact staged model preview. No grants consumed, revisions committed or artifacts attached.
#[cfg(not(feature = "native-occt"))]
use crate::core::AppError;
use crate::core::{AppState, Result};
use std::sync::atomic::Ordering;
use tauri::State;
#[cfg(feature = "native-occt")]
pub mod build;
#[cfg(feature = "native-occt")]
mod host;
#[cfg(feature = "native-occt")]
mod imports;
#[cfg(feature = "native-occt")]
mod packet;
#[cfg(feature = "native-occt")]
mod workspace;
#[tauri::command]
pub async fn preview_model(
    project_id: String,
    program: String,
    expected_revision: Option<String>,
    state: State<'_, AppState>,
) -> Result<tauri::ipc::Response> {
    #[cfg(not(feature = "native-occt"))]
    {
        let _ = (project_id, program, expected_revision, state);
        Err(AppError::Invalid(
            "Model preview requires the native OpenCascade build".into(),
        ))
    }
    #[cfg(feature = "native-occt")]
    {
        host::preview(&state, &project_id, &program, expected_revision)
            .await
            .map(tauri::ipc::Response::new)
    }
}
#[tauri::command]
pub async fn cancel_model_preview(project_id: String, state: State<'_, AppState>) -> Result<()> {
    crate::security::valid_id(&project_id)?;
    if let Some(cancel) = state
        .tasks
        .lock()
        .await
        .get(&format!("preview:{project_id}"))
    {
        cancel.store(true, Ordering::Relaxed);
    }
    Ok(())
}
