use super::WindowSources;
use crate::core::{AppError, Result};
use tauri::State;

#[derive(serde::Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum FolderWriteAvailability {
    Available,
    ReadOnly { reason: String },
}

#[tauri::command]
pub async fn project_folder_write_availability(
    project_id: String,
    source_id: String,
    expected_commit_id: String,
    window: tauri::WebviewWindow,
    sources: State<'_, WindowSources>,
) -> Result<FolderWriteAvailability> {
    #[cfg(any(
        windows,
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        )
    ))]
    {
        let operation = sources
            .capture_operation(window.label(), &source_id, &project_id, &expected_commit_id)
            .await?;
        match operation.check_write_available().await {
            Ok(()) => Ok(FolderWriteAvailability::Available),
            Err(AppError::Invalid(reason))
                if matches!(
                    reason.as_str(),
                    "PROJECT_READ_ONLY" | "PROJECT_ACCESS_UNAVAILABLE"
                ) =>
            {
                operation.recheck().await?;
                Ok(FolderWriteAvailability::ReadOnly { reason })
            }
            Err(error) => Err(error),
        }
    }
    #[cfg(not(any(
        windows,
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        )
    )))]
    {
        let _ = (project_id, source_id, expected_commit_id, window, sources);
        Err(AppError::Invalid("PROJECT_FOLDER_UNSUPPORTED".into()))
    }
}
