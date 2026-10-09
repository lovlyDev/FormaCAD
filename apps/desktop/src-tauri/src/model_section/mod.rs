//! Read-only section of the committed revision; no model edit or export side effect.
use crate::core::{AppState, Result};
#[cfg(feature = "native-occt")]
pub mod build;
#[cfg(feature = "native-occt")]
mod workspace;
#[cfg(feature = "native-occt")]
pub use crate::native::section::{SectionPlane, SectionReport};

#[tauri::command]
pub async fn section_model(
    project_id: String,
    expected_revision: String,
    origin_mm: [f64; 3],
    normal: [f64; 3],
    deflection_mm: Option<f64>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value> {
    #[cfg(not(feature = "native-occt"))]
    {
        let _ = (
            project_id,
            expected_revision,
            origin_mm,
            normal,
            deflection_mm,
            state,
        );
        Err(failure(
            "SECTION_UNAVAILABLE",
            "Model section requires the native OpenCascade build",
        ))
    }
    #[cfg(feature = "native-occt")]
    {
        let plane = SectionPlane {
            origin_mm,
            normal,
            deflection_mm: deflection_mm.unwrap_or(0.01),
        };
        let executable = crate::native::worker::worker_executable()?;
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
