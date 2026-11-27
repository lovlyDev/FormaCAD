//! Read-only section of the committed revision; no model edit or export side effect.
use crate::core::{AppState, Result};
#[cfg(feature = "native-worker-client")]
pub mod build;
#[cfg(feature = "native-worker-client")]
mod workspace;
#[cfg(feature = "native-worker-client")]
pub use crate::native::section::{SectionPlane, SectionReport};

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn section_model(
    project_id: String,
    expected_revision: String,
    origin_mm: [f64; 3],
    normal: [f64; 3],
    deflection_mm: Option<f64>,
    source_id: Option<String>,
    expected_commit_id: Option<String>,
    window: tauri::WebviewWindow,
    sources: tauri::State<'_, crate::project_repository::window_sources::WindowSources>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value> {
    if source_id.is_some() != expected_commit_id.is_some() {
        return Err(crate::core::AppError::Invalid(
            "PROJECT_SNAPSHOT_CHANGED".into(),
        ));
    }
    #[cfg(not(feature = "native-worker-client"))]
    {
        let _ = (
            project_id,
            expected_revision,
            origin_mm,
            normal,
            deflection_mm,
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
            "Model section requires the native OpenCascade build",
        ))
    }
    #[cfg(feature = "native-worker-client")]
    {
        let plane = SectionPlane {
            origin_mm,
            normal,
            deflection_mm: deflection_mm.unwrap_or(0.01),
        };
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
            let report = build::section_source_with_executable(
                &state,
                read,
                &expected_revision,
                &plane,
                std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
                &executable,
            )
            .await?;
            return Ok(serde_json::to_value(report)?);
        }
        let report = build::section_with_executable(
            &state,
            &project_id,
            &expected_revision,
            &plane,
            std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            &executable,
        )
        .await?;
        Ok(serde_json::to_value(report)?)
    }
}
pub(super) fn failure(code: &str, detail: &str) -> crate::core::AppError {
    crate::core::AppError::Invalid(serde_json::json!({"code":code,"message":detail}).to_string())
}
