use super::WindowSources;
use crate::core::{AppError, Result};
use tauri::State;

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DraftSource {
    project_id: String,
    source_id: String,
    expected_commit_id: String,
}

#[tauri::command]
pub async fn keep_folder_editor_draft(
    source: DraftSource,
    editor_text: String,
    window: tauri::WebviewWindow,
    sources: State<'_, WindowSources>,
) -> Result<()> {
    #[cfg(any(
        windows,
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        )
    ))]
    {
        let (operation, store) = draft_context(source, &window, &sources).await?;
        operation.keep_local_editor_draft(&store, editor_text).await
    }
    #[cfg(not(any(
        windows,
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        )
    )))]
    {
        let DraftSource {
            project_id,
            source_id,
            expected_commit_id,
        } = source;
        let _ = (
            project_id,
            source_id,
            expected_commit_id,
            editor_text,
            window,
            sources,
        );
        Err(AppError::Invalid("PROJECT_FOLDER_UNSUPPORTED".into()))
    }
}

#[tauri::command]
pub async fn review_folder_editor_draft(
    source: DraftSource,
    window: tauri::WebviewWindow,
    sources: State<'_, WindowSources>,
) -> Result<Option<String>> {
    #[cfg(any(
        windows,
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        )
    ))]
    {
        let (operation, store) = draft_context(source, &window, &sources).await?;
        operation.review_local_editor_draft(&store).await
    }
    #[cfg(not(any(
        windows,
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        )
    )))]
    {
        let _ = (source, window, sources);
        Err(AppError::Invalid("PROJECT_FOLDER_UNSUPPORTED".into()))
    }
}

#[cfg(any(
    windows,
    all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )
))]
async fn draft_context(
    source: DraftSource,
    window: &tauri::WebviewWindow,
    sources: &WindowSources,
) -> Result<(
    crate::project_repository::window_sources::operation::FolderOperation,
    crate::project_repository::sqlite_folder::api::LocalDraftStore,
)> {
    use tauri::Manager;
    let captured = sources
        .capture_operation(
            window.label(),
            &source.source_id,
            &source.project_id,
            &source.expected_commit_id,
        )
        .await?;
    let app_data = window
        .app_handle()
        .path()
        .app_data_dir()
        .map_err(|error| AppError::Invalid(error.to_string()))?;
    Ok((
        captured,
        crate::project_repository::sqlite_folder::api::LocalDraftStore::open(&app_data)?,
    ))
}
