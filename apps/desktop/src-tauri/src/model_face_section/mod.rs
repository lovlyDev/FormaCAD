//! Single atomic read-only request; only the native worker derives the plane.
#[cfg(feature = "native-occt")]
pub mod build;
pub mod schema;
#[cfg(feature = "native-occt")]
mod workspace;
use crate::core::{AppError, AppState, Result};
use schema::{FaceSectionQuery, FaceSectionReport};

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmedFaceSection {
    pub project_id: String,
    #[serde(flatten)]
    pub report: FaceSectionReport,
}
pub(super) fn failure(code: &str, detail: impl Into<String>) -> AppError {
    AppError::Invalid(serde_json::json!({"code":code,"message":detail.into()}).to_string())
}
#[tauri::command]
pub async fn section_model_reference(
    project_id: String,
    expected_revision: String,
    body_id: String,
    query: FaceSectionQuery,
    state: tauri::State<'_, AppState>,
) -> Result<ConfirmedFaceSection> {
    #[cfg(feature = "native-occt")]
    {
        let executable = crate::native::worker::worker_executable()?;
        build::section_with_executable(
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
            "SECTION_UNAVAILABLE",
            "Face section requires the native OpenCascade build",
        ))
    }
}
