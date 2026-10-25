//! Read exact CAD properties for any saved STEP revision in the project.
use crate::core::{AppState, Result};
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
    state: State<'_, AppState>,
) -> Result<Option<ExactModelProperties>> {
    #[cfg(not(feature = "native-worker-client"))]
    {
        let _ = (project_id, revision_id, state);
        Ok(None)
    }
    #[cfg(feature = "native-worker-client")]
    {
        read::inspect(&state, &project_id, &revision_id).await
    }
}
