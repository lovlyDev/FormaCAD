//! Read exact CAD properties for any saved STEP revision in the project.
use crate::core::{AppError, AppState, Result};
#[cfg(feature = "native-worker-client")]
mod read;
use tauri::State;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExactModelProperties {
    pub volume_mm3: f64,
    pub area_mm2: f64,
    pub face_count: i32,
    pub edge_count: i32,
    pub bounds_mm: [f64; 3],
}

#[tauri::command]
pub async fn inspect_model(
    project_id: String,
    revision_id: String,
    source_id: Option<String>,
    expected_commit_id: Option<String>,
    window: tauri::WebviewWindow,
    sources: State<'_, crate::project_repository::window_sources::WindowSources>,
    state: State<'_, AppState>,
) -> Result<Option<ExactModelProperties>> {
    if source_id.is_some() != expected_commit_id.is_some() {
        return Err(AppError::Invalid("PROJECT_SNAPSHOT_CHANGED".into()));
    }
    #[cfg(not(feature = "native-worker-client"))]
    {
        let _ = (
            project_id,
            revision_id,
            window,
            sources,
            state,
            expected_commit_id,
        );
        if source_id.is_some() {
            return Err(AppError::Invalid("PROJECT_FOLDER_UNSUPPORTED".into()));
        }
        Ok(None)
    }
    #[cfg(feature = "native-worker-client")]
    {
        match (source_id, expected_commit_id) {
            (None, None) => read::inspect(&state, &project_id, &revision_id).await,
            (Some(source), Some(commit)) => {
                let captured = sources
                    .capture_read(window.label(), &source, &project_id, &commit)
                    .await?;
                read::inspect_bound(&state, captured, &revision_id).await
            }
            _ => Err(AppError::Invalid("PROJECT_SNAPSHOT_CHANGED".into())),
        }
    }
}
