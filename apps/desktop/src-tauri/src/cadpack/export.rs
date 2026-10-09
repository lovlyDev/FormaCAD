//! Export a single checked committed model/history snapshot without source recovery.
use super::{
    invalid, picked_path, redaction, write_bundle_with, MAX_ARCHIVE_BYTES, MAX_FILES,
    MAX_PROJECT_BYTES,
};
use crate::{
    core::{AppState, Result},
    project_repository::legacy,
};
use std::{collections::HashMap, path::Path};
use tauri::State;
use tauri_plugin_dialog::DialogExt;

/// Prepare immutable bytes and recheck their captured authority before touching
/// the destination. The existing bundle writer publishes only a finished archive.
pub(super) async fn write_snapshot(
    state: &AppState,
    project_id: &str,
    destination: &Path,
    redact_conversation: bool,
) -> Result<()> {
    let snapshot = legacy::capture_with_history(state, project_id).await?;
    let project = if redact_conversation {
        redaction::conversation_free(&snapshot.project)?
    } else {
        snapshot.project.clone()
    };
    let history = snapshot
        .history
        .as_deref()
        .ok_or_else(|| invalid("HISTORY_INVALID_STATE"))?;
    if project.files.len() > MAX_FILES {
        return Err(invalid("Project has too many attachments for a bundle"));
    }
    let project_bytes = serde_json::to_vec(&project)?;
    if project_bytes.len() as u64 > MAX_PROJECT_BYTES {
        return Err(invalid("Project metadata exceeds bundle limit"));
    }
    let mut total = project_bytes.len() as u64 + history.len() as u64;
    let mut attachments = HashMap::<String, Vec<u8>>::new();
    for entry in &project.files {
        let hash = entry
            .sha256
            .as_ref()
            .ok_or_else(|| invalid("Project attachment checksum is missing"))?;
        if let Some(bytes) = attachments.get(hash) {
            if bytes.len() as u64 != entry.size {
                return Err(invalid("Project attachment size mismatch"));
            }
            continue;
        }
        total = total
            .checked_add(entry.size)
            .ok_or_else(|| invalid("Project bundle exceeds 150 MB"))?;
        if total > MAX_ARCHIVE_BYTES {
            return Err(invalid("Project bundle exceeds 150 MB"));
        }
        let bytes = crate::artifacts::read(state, project_id, entry)?;
        if bytes.len() as u64 != entry.size || crate::artifacts::digest(&bytes) != *hash {
            return Err(invalid("Project attachment size mismatch"));
        }
        attachments.insert(hash.clone(), bytes);
    }
    legacy::recheck(state, &snapshot).await?;
    write_bundle_with(&project, destination, Some(history), |entry| {
        attachments
            .remove(entry.sha256.as_deref().unwrap_or_default())
            .ok_or_else(|| invalid("Project attachment checksum is missing"))
    })
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
    let suggested = format!(
        "{}.cadpack",
        snapshot
            .project
            .name
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
            .take(60)
            .collect::<String>()
    );
    drop(snapshot);
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
    let Some(target) = target else {
        return Ok(None);
    };
    let path = picked_path(target)?;
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

#[cfg(test)]
#[path = "export_tests.rs"]
mod tests;
