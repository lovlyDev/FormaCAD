//! Export a single checked committed model/history snapshot without source recovery.
use super::{invalid, picked_path};
use crate::{
    core::{AppState, Result},
    project_repository::legacy,
};
use std::{collections::HashMap, path::Path};
use tauri::State;
use tauri_plugin_dialog::DialogExt;

pub mod folder;
mod prepared;

/// Prepare immutable bytes and recheck their captured authority before touching
/// the destination. The existing bundle writer publishes only a finished archive.
pub(super) async fn write_snapshot(
    state: &AppState,
    project_id: &str,
    destination: &Path,
    redact_conversation: bool,
) -> Result<()> {
    let snapshot = legacy::capture_with_history(state, project_id).await?;
    let history = snapshot
        .history
        .as_deref()
        .ok_or_else(|| invalid("HISTORY_INVALID_STATE"))?;
    let bundle = prepared::BundlePlan::new(&snapshot.project, history, redact_conversation)?;
    let mut attachments = HashMap::new();
    for (hash, entry) in bundle.attachments() {
        attachments.insert(hash, crate::artifacts::read(state, project_id, entry)?);
    }
    legacy::recheck(state, &snapshot).await?;
    bundle.write(destination, attachments)
}

#[tauri::command]
pub async fn export_project_bundle(
    project_id: String,
    locale: Option<String>,
    redact_conversation: Option<bool>,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<Option<String>> {
    let snapshot = legacy::capture_project(&state, &project_id).await?;
    crate::permissions::consume(&state, &project_id, "export_file").await?;
    let suggested = suggested_bundle_name(&snapshot.project.name);
    drop(snapshot);
    let Some(path) = pick_bundle_destination(app, suggested, locale).await? else {
        return Ok(None);
    };
    let _guard = state.writes.lock().await;
    write_snapshot(
        &state,
        &project_id,
        &path,
        redact_conversation.unwrap_or(false),
    )
    .await?;
    Ok(Some(path.display().to_string()))
}

pub(super) fn suggested_bundle_name(project_name: &str) -> String {
    format!(
        "{}.cadpack",
        project_name
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
            .take(60)
            .collect::<String>()
    )
}

pub(super) async fn pick_bundle_destination(
    app: tauri::AppHandle,
    suggested: String,
    locale: Option<String>,
) -> Result<Option<std::path::PathBuf>> {
    let target = tokio::task::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title(if locale.as_deref() == Some("en") {
                "Export project bundle"
            } else {
                "Экспорт пакета проекта"
            })
            .set_file_name(&suggested)
            .add_filter("Forma CAD", &["cadpack"])
            .blocking_save_file()
    })
    .await
    .map_err(|error| invalid(error.to_string()))?;
    target.map(picked_path).transpose()
}

#[cfg(test)]
#[path = "export_tests.rs"]
mod tests;
