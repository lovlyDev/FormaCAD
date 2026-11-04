//! Explicit selection/close; individual reads use the existing read_project_snapshot IPC.
use super::WindowSources;
use crate::{core::Result, models::Project};
use tauri::State;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderOpen {
    source_id: String,
    commit_id: String,
    project: Project,
}

#[tauri::command]
pub async fn open_project_folder(
    window: tauri::WebviewWindow,
    sources: State<'_, WindowSources>,
) -> Result<Option<FolderOpen>> {
    #[cfg(any(
        windows,
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        )
    ))]
    {
        use tauri::Manager;
        use tauri_plugin_dialog::DialogExt;
        // Reserve the original window scope BEFORE the dialog await. Destroying
        // this window closes it even if a late picker result arrives afterward.
        let scope = sources.for_window(window.label())?;
        let app = window.app_handle().clone();
        let picked = tokio::task::spawn_blocking(move || {
            app.dialog()
                .file()
                .set_title("Forma CAD")
                .blocking_pick_folder()
        })
        .await
        .map_err(|error| crate::core::AppError::Invalid(error.to_string()))?;
        let Some(picked) = picked else {
            return Ok(None);
        };
        let path = picked
            .into_path()
            .map_err(|error| crate::core::AppError::Invalid(error.to_string()))?;
        let opened = scope.open(&path).await?;
        // No further await after the registry's cancellation-protected recheck.
        Ok(Some(FolderOpen {
            source_id: opened.id.to_string(),
            commit_id: opened.snapshot.commit_id().into(),
            project: opened.snapshot.project().clone(),
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
        let _ = (window, sources);
        Err(crate::core::AppError::Invalid(
            "PROJECT_FOLDER_UNSUPPORTED".into(),
        ))
    }
}

#[tauri::command]
pub fn close_project_source(
    source_id: String,
    window: tauri::WebviewWindow,
    sources: State<'_, WindowSources>,
) -> Result<bool> {
    sources.close_source(window.label(), &source_id)
}
