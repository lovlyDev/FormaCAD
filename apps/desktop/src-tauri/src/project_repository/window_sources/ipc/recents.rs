use super::WindowSources;
use crate::core::{AppError, Result};
use std::path::{Path, PathBuf};
use tauri::State;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentFolderItem {
    location: PathBuf,
    last_opened_at_ms: i64,
    pinned: bool,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentFolders {
    items: Vec<RecentFolderItem>,
    remember_recent: bool,
}

#[tauri::command]
pub async fn list_recent_folders(
    sources: State<'_, WindowSources>,
) -> Result<Option<RecentFolders>> {
    let Some(index) = &sources.index else {
        return Ok(None);
    };
    Ok(Some(RecentFolders {
        items: index
            .recent()
            .await?
            .into_iter()
            .map(|row| RecentFolderItem {
                location: row.location,
                last_opened_at_ms: row.last_opened_at_ms,
                pinned: row.pinned,
            })
            .collect(),
        remember_recent: index.remember_recent().await?,
    }))
}

#[tauri::command]
pub async fn pin_recent_folder(
    location: String,
    pinned: bool,
    sources: State<'_, WindowSources>,
) -> Result<bool> {
    sources
        .index
        .as_ref()
        .ok_or_else(unsupported)?
        .set_pinned(Path::new(&location), pinned)
        .await
}

#[tauri::command]
pub async fn remember_recent_folders(
    enabled: bool,
    sources: State<'_, WindowSources>,
) -> Result<()> {
    sources
        .index
        .as_ref()
        .ok_or_else(unsupported)?
        .set_remember_recent(enabled)
        .await
}

fn unsupported() -> AppError {
    AppError::Invalid("PROJECT_FOLDER_UNSUPPORTED".into())
}

#[cfg(any(
    windows,
    all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )
))]
pub(super) async fn picker_location(
    sources: &WindowSources,
    requested: Option<&str>,
) -> Option<PathBuf> {
    let requested = Path::new(requested?);
    let items = sources.index.as_ref()?.recent().await.ok()?;
    items
        .into_iter()
        .find(|row| row.location == requested)
        .map(|row| row.location)
}
