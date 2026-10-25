//! Read-only committed-authored-body measurements. Schema is supplied by native stage.
#[cfg(feature = "native-worker-client")]
pub mod build;
#[cfg(feature = "native-worker-client")]
pub(crate) mod capture;
pub mod schema;
#[cfg(feature = "native-worker-client")]
mod snapshot;
#[cfg(feature = "native-worker-client")]
mod workspace;

use crate::core::{AppError, AppState, Result};
use schema::{MeasurementQuery, MeasurementReport};

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmedMeasurement {
    pub project_id: String,
    #[serde(flatten)]
    pub report: MeasurementReport,
}

#[tauri::command]
pub async fn measure_model_reference(
    project_id: String,
    expected_revision: String,
    body_id: String,
    query: MeasurementQuery,
    state: tauri::State<'_, AppState>,
) -> Result<ConfirmedMeasurement> {
    #[cfg(feature = "native-worker-client")]
    {
        let executable = crate::native::worker::worker_executable()?;
        build::measure_with_executable(
            &state,
            &project_id,
            &expected_revision,
            &body_id,
            &query,
            std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            &executable,
        )
        .await
    }
    #[cfg(not(feature = "native-worker-client"))]
    {
        let _ = (project_id, expected_revision, body_id, query, state);
        Err(failure(
            "MEASUREMENT_UNAVAILABLE",
            "Exact reference measurements require the native build",
        ))
    }
}

pub(super) fn failure(code: &str, detail: impl Into<String>) -> AppError {
    AppError::Invalid(serde_json::json!({"code":code,"message":detail.into()}).to_string())
}
