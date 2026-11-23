use super::{creation::FolderCreationReply, WindowSources};
use crate::core::Result;
use tauri::State;

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryPickerTitles {
    source: String,
    destination: String,
}

#[tauri::command]
pub async fn recover_project_folder_copy(
    titles: RecoveryPickerTitles,
    window: tauri::WebviewWindow,
    sources: State<'_, WindowSources>,
) -> Result<Option<FolderCreationReply>> {
    #[cfg(any(
        windows,
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        )
    ))]
    {
        use super::pick_folder_with_title;
        use crate::core::AppError;
        use tauri::Manager;

        let scope = sources.for_window(window.label())?;
        let Some(source) = pick_folder_with_title(&window, None, titles.source).await? else {
            return Ok(None);
        };
        if !std::sync::Arc::ptr_eq(&scope, &sources.existing(window.label())?) {
            return Err(super::super::changed());
        }
        let Some(destination) = pick_folder_with_title(&window, None, titles.destination).await?
        else {
            return Ok(None);
        };
        let scratch = window
            .app_handle()
            .path()
            .app_data_dir()
            .map_err(|error| AppError::Invalid(error.to_string()))?;
        let recovered = scope.recover_copy(&source, &destination, &scratch).await?;
        Ok(Some(recovered.into()))
    }
    #[cfg(not(any(
        windows,
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        )
    )))]
    {
        let _ = (window, sources, titles.source, titles.destination);
        Err(crate::core::AppError::Invalid(
            "PROJECT_FOLDER_UNSUPPORTED".into(),
        ))
    }
}
