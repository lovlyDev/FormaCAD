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
#[allow(clippy::too_many_arguments)]
pub async fn snap_model_reference(
    project_id: String,
    expected_revision: String,
    body_id: String,
    query: SnapQuery,
    source_id: Option<String>,
    expected_commit_id: Option<String>,
    window: tauri::WebviewWindow,
    sources: tauri::State<'_, crate::project_repository::window_sources::WindowSources>,
    state: tauri::State<'_, AppState>,
) -> Result<ConfirmedSnap> {
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
            return build::snap_source_with_executable(
                &state,
                read,
                &expected_revision,
                &body_id,
                &query,
                &executable,
            )
            .await;
        }
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
        Err(crate::core::AppError::Invalid(
            serde_json::json!({"code":"SNAP_UNAVAILABLE","message":"Exact snapping requires the native build"})
                .to_string(),
        ))
    }
}
