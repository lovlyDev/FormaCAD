//! Single atomic read-only request; only the native worker derives the plane.
#[cfg(feature = "native-worker-client")]
pub mod build;
pub mod schema;
#[cfg(feature = "native-worker-client")]
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
#[allow(clippy::too_many_arguments)]
pub async fn section_model_reference(
    project_id: String,
    expected_revision: String,
    body_id: String,
    query: FaceSectionQuery,
    source_id: Option<String>,
    expected_commit_id: Option<String>,
    window: tauri::WebviewWindow,
    sources: tauri::State<'_, crate::project_repository::window_sources::WindowSources>,
    state: tauri::State<'_, AppState>,
) -> Result<ConfirmedFaceSection> {
    if source_id.is_some() != expected_commit_id.is_some() {
        return Err(crate::core::AppError::Invalid(
            "PROJECT_SNAPSHOT_CHANGED".into(),
        ));
    }
    #[cfg(feature = "native-worker-client")]
    {
        let source = match (source_id, expected_commit_id) {
            (Some(key), Some(commit)) => Some(
                sources
                    .capture_read(window.label(), &key, &project_id, &commit)
                    .await?,
            ),
            (None, None) => None,
            _ => {
                return Err(crate::core::AppError::Invalid(
                    "PROJECT_SNAPSHOT_CHANGED".into(),
                ))
            }
        };
        let executable = crate::native::worker::worker_executable()?;
        if let Some(read) = source {
            return build::section_source_with_executable(
                &state,
                read,
                &expected_revision,
                &body_id,
                &query,
                std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
                &executable,
            )
            .await;
        }
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
    #[cfg(not(feature = "native-worker-client"))]
    {
        let _ = (
            project_id,
            expected_revision,
            body_id,
            query,
            state,
            window,
            sources,
            expected_commit_id,
        );
        if source_id.is_some() {
            return Err(crate::core::AppError::Invalid(
                "PROJECT_FOLDER_UNSUPPORTED".into(),
            ));
        }
        Err(failure(
            "SECTION_UNAVAILABLE",
            "Face section requires the native OpenCascade build",
        ))
    }
}
