//! Explicit selection/close; individual reads use the existing read_project_snapshot IPC.
use super::WindowSources;
use crate::{core::Result, models::Project};
use tauri::State;

pub mod access;
pub mod commit_query;
pub mod copy;
pub mod creation;
pub mod recents;
pub mod recovery;
pub mod step;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderOpen {
    source_id: String,
    commit_id: String,
    task_scope_id: String,
    project: Project,
}

#[tauri::command]
pub async fn open_project_folder(
    recent_location: Option<String>,
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
        // Reserve the original window scope BEFORE the dialog await. Destroying
        // this window closes it even if a late picker result arrives afterward.
        let scope = sources.for_window(window.label())?;
        let initial = recents::picker_location(&sources, recent_location.as_deref()).await;
        let Some(path) = pick_folder(&window, initial).await? else {
            return Ok(None);
        };
        let opened = scope.open(&path).await?;
        // No further await after the registry's cancellation-protected recheck.
        Ok(Some(folder_open(opened)))
    }
    #[cfg(not(any(
        windows,
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        )
    )))]
    {
        let _ = (recent_location, window, sources);
        Err(crate::core::AppError::Invalid(
            "PROJECT_FOLDER_UNSUPPORTED".into(),
        ))
    }
}

#[cfg(any(
    windows,
    all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )
))]
async fn pick_folder(
    window: &tauri::WebviewWindow,
    initial: Option<std::path::PathBuf>,
) -> Result<Option<std::path::PathBuf>> {
    pick_folder_with_title(window, initial, "Forma CAD".into()).await
}

#[cfg(any(
    windows,
    all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )
))]
async fn pick_folder_with_title(
    window: &tauri::WebviewWindow,
    initial: Option<std::path::PathBuf>,
    title: String,
) -> Result<Option<std::path::PathBuf>> {
    use crate::core::AppError;
    use tauri::Manager;
    use tauri_plugin_dialog::DialogExt;
    let app = window.app_handle().clone();
    let picked = tokio::task::spawn_blocking(move || {
        let dialog = app.dialog().file().set_title(title);
        match initial {
            Some(initial) => dialog.set_directory(initial).blocking_pick_folder(),
            None => dialog.blocking_pick_folder(),
        }
    })
    .await
    .map_err(|error| AppError::Invalid(error.to_string()))?;
    picked
        .map(|path| {
            path.into_path()
                .map_err(|error| AppError::Invalid(error.to_string()))
        })
        .transpose()
}

#[cfg(any(
    windows,
    all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )
))]
fn folder_open(opened: crate::project_repository::sqlite_folder::api::OpenedFolder) -> FolderOpen {
    FolderOpen {
        source_id: opened.id.to_string(),
        commit_id: opened.snapshot.commit_id().into(),
        task_scope_id: opened.snapshot.task_scope_id().into(),
        project: opened.snapshot.project().clone(),
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
