//! Read-only, same-body exact pair measurements; no pins/constraints/history writes.
#[cfg(feature = "native-occt")]
pub mod build;
pub mod schema;
#[cfg(feature = "native-occt")]
mod workspace;
use crate::core::{AppError, AppState, Result};
use schema::{PairMeasurementQuery, PairMeasurementReport};
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmedPairMeasurement {
    pub project_id: String,
    #[serde(flatten)]
    pub report: PairMeasurementReport,
}
pub(super) fn failure(code: &str, detail: impl Into<String>) -> AppError {
    AppError::Invalid(serde_json::json!({"code":code,"message":detail.into()}).to_string())
}
#[tauri::command]
pub async fn measure_model_pair(
    project_id: String,
    expected_revision: String,
    body_id: String,
    query: PairMeasurementQuery,
    state: tauri::State<'_, AppState>,
) -> Result<ConfirmedPairMeasurement> {
    #[cfg(feature = "native-occt")]
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
    #[cfg(not(feature = "native-occt"))]
    {
        let _ = (project_id, expected_revision, body_id, query, state);
        Err(failure(
            "MEASUREMENT_UNAVAILABLE",
            "Pair measurements require the native OpenCascade build",
        ))
    }
}
