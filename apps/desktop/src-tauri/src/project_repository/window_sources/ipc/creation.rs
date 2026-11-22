use super::{FolderOpen, WindowSources};
use crate::core::{AppError, Result};
use tauri::State;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BlankFolderProject {
    name: String,
    units: String,
    agent: String,
}

#[derive(serde::Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum FolderCreationReply {
    Opened { source: Box<FolderOpen> },
    SavedWithoutOpening { commit_id: String, error: AppError },
    Indeterminate { commit_id: String, error: AppError },
}

#[cfg(any(
    windows,
    all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )
))]
impl From<crate::project_repository::sqlite_folder::api::FolderCreation> for FolderCreationReply {
    fn from(creation: crate::project_repository::sqlite_folder::api::FolderCreation) -> Self {
        use crate::project_repository::sqlite_folder::api::FolderCreation;
        match creation {
            FolderCreation::Opened(opened) => Self::Opened {
                source: Box::new(super::folder_open(*opened)),
            },
            FolderCreation::SavedWithoutOpening { commit_id, error } => {
                Self::SavedWithoutOpening { commit_id, error }
            }
            FolderCreation::Indeterminate { commit_id, error } => {
                Self::Indeterminate { commit_id, error }
            }
        }
    }
}

#[tauri::command]
pub async fn create_project_folder(
    draft: BlankFolderProject,
    window: tauri::WebviewWindow,
    sources: State<'_, WindowSources>,
) -> Result<Option<FolderCreationReply>> {
    let BlankFolderProject { name, units, agent } = draft;
    #[cfg(any(
        windows,
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        )
    ))]
    {
        use super::pick_folder;
        let scope = sources.for_window(window.label())?;
        let Some(path) = pick_folder(&window, None).await? else {
            return Ok(None);
        };
        let created = scope.create_blank(&path, name, units, agent).await?;
        Ok(Some(created.into()))
    }
    #[cfg(not(any(
        windows,
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        )
    )))]
    {
        let _ = (name, units, agent, window, sources);
        Err(AppError::Invalid("PROJECT_FOLDER_UNSUPPORTED".into()))
    }
}
