//! Exact snap wire contracts and the read-only host route for committed bodies.
#[cfg(feature = "native-worker-client")]
pub mod build;
#[cfg(feature = "native-worker-client")]
mod client;
pub mod report;
pub mod schema;

use crate::core::{AppState, Result};
use schema::{SnapQuery, SnapReport};

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmedSnap {
    pub project_id: String,
    #[serde(flatten)]
    pub report: SnapReport,
}

#[tauri::command]
pub async fn snap_model_reference(
    project_id: String,
    expected_revision: String,
    body_id: String,
    query: SnapQuery,
    state: tauri::State<'_, AppState>,
) -> Result<ConfirmedSnap> {
    #[cfg(feature = "native-worker-client")]
    {
        let executable = crate::native::worker::worker_executable()?;
        build::snap_with_executable(
            &state,
            &project_id,
            &expected_revision,
            &body_id,
            &query,
            &executable,
        )
        .await
    }
    #[cfg(not(feature = "native-worker-client"))]
    {
        let _ = (project_id, expected_revision, body_id, query, state);
        Err(crate::core::AppError::Invalid(
            serde_json::json!({"code":"SNAP_UNAVAILABLE","message":"Exact snapping requires the native build"})
                .to_string(),
        ))
    }
}
