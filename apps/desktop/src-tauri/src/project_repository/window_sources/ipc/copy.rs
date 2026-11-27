use super::WindowSources;
use crate::core::{AppError, Result};
use tauri::State;

#[derive(serde::Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum FolderCopyReply {
    Saved { commit_id: String },
    Indeterminate { commit_id: String, error: AppError },
}

#[tauri::command]
pub async fn save_project_folder_copy(
    project_id: String,
    source_id: String,
    expected_commit_id: String,
    window: tauri::WebviewWindow,
    sources: State<'_, WindowSources>,
) -> Result<Option<FolderCopyReply>> {
    #[cfg(any(
        windows,
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        )
    ))]
    {
        use super::pick_folder;
        use crate::project_repository::sqlite_folder::api::Publication;
        let operation = sources
            .capture_operation(window.label(), &source_id, &project_id, &expected_commit_id)
            .await?;
        let Some(destination) = pick_folder(&window, None).await? else {
            return Ok(None);
        };
        Ok(Some(match operation.save_copy(&destination).await? {
            Publication::Committed(saved) => FolderCopyReply::Saved {
                commit_id: saved.commit_id().to_owned(),
            },
            Publication::RejectedBefore(error) => return Err(error),
            Publication::Indeterminate { commit_id, error } => {
                FolderCopyReply::Indeterminate { commit_id, error }
            }
        }))
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
