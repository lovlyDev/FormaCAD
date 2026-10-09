//! Portable, self-contained project snapshots. The archive contains only the
//! project document and the immutable attachments it references.
pub mod export;
mod history;
mod redaction;
use crate::{
    core::{AppError, AppState, Result},
    models::Project,
};
pub use export::export_project_bundle;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
};
use tauri::State;
use tauri_plugin_dialog::DialogExt;
use zip::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};

const MAX_ARCHIVE_BYTES: u64 = 150 * 1024 * 1024;
const MAX_PROJECT_BYTES: u64 = 16 * 1024 * 1024;
const MAX_FILES: usize = 2048;
type BundleContents = (Project, Vec<(String, Vec<u8>)>, Option<Vec<u8>>);

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Manifest {
    format: String,
    format_version: u32,
    project_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    history: Option<history::HistoryEntry>,
}

fn invalid(detail: impl Into<String>) -> AppError {
    AppError::Invalid(detail.into())
}

fn zip_error(error: zip::result::ZipError) -> AppError {
    invalid(format!("Invalid project bundle: {error}"))
}

fn attachment_path(hash: &str) -> String {
    format!("attachments/{hash}")
}

fn write_bundle_with(
    project: &Project,
    destination: &Path,
    model_history: Option<&[u8]>,
    mut read_attachment: impl FnMut(&crate::models::ProjectFile) -> Result<Vec<u8>>,
) -> Result<()> {
    if project.files.len() > MAX_FILES {
        return Err(invalid("Project has too many attachments for a bundle"));
    }
    let project_bytes = serde_json::to_vec(project)?;
    if project_bytes.len() as u64 > MAX_PROJECT_BYTES {
        return Err(invalid("Project metadata exceeds bundle limit"));
    }
    if let Some(bytes) = model_history {
        crate::project_history::validate_archive(project, bytes)?;
    }
    let temp = destination.with_extension(format!("cadpack-{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        let mut archive = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        archive
            .start_file("manifest.json", options)
            .map_err(zip_error)?;
        archive.write_all(&serde_json::to_vec(&Manifest {
            format: "Forma CAD project".into(),
            format_version: if model_history.is_some() { 2 } else { 1 },
            project_id: project.id.clone(),
            history: model_history.map(history::descriptor).transpose()?,
        })?)?;
        archive
            .start_file("project.json", options)
            .map_err(zip_error)?;
        archive.write_all(&project_bytes)?;
        if let Some(bytes) = model_history {
            archive
                .start_file("history.json", options)
                .map_err(zip_error)?;
            archive.write_all(bytes)?;
        }
        let mut written = std::collections::HashSet::new();
        for entry in &project.files {
            let hash = entry
                .sha256
                .as_ref()
                .ok_or_else(|| invalid("Project attachment checksum is missing"))?;
            if !written.insert(hash.clone()) {
                continue;
            }
            let bytes = read_attachment(entry)?;
            if bytes.len() as u64 != entry.size || crate::artifacts::digest(&bytes) != *hash {
                return Err(invalid("Project attachment size mismatch"));
            }
            archive
                .start_file(attachment_path(hash), options)
                .map_err(zip_error)?;
            archive.write_all(&bytes)?;
        }
        let file = archive.finish().map_err(zip_error)?;
        file.sync_all()?;
        if std::fs::metadata(&temp)?.len() > MAX_ARCHIVE_BYTES {
            return Err(invalid("Project bundle exceeds 150 MB"));
        }
        // Keep the previous bundle intact until the replacement is fully written.
        let backup = destination.with_extension(format!("cadpack-{}.bak", uuid::Uuid::new_v4()));
        let had_previous = destination.exists();
        if had_previous {
            std::fs::rename(destination, &backup)?;
        }
        if let Err(error) = std::fs::rename(&temp, destination) {
            if had_previous {
                let _ = std::fs::rename(&backup, destination);
            }
            return Err(error.into());
        }
        if had_previous {
            let _ = std::fs::remove_file(backup);
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result
}

fn read_limited(file: &mut impl Read, limit: u64) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(invalid("Project bundle entry is too large"));
    }
    Ok(bytes)
}

fn read_bundle(path: &Path) -> Result<BundleContents> {
    let file = File::open(path)?;
    if file.metadata()?.len() > MAX_ARCHIVE_BYTES {
        return Err(invalid("Project bundle exceeds 150 MB"));
    }
    let mut archive = ZipArchive::new(file).map_err(zip_error)?;
    if archive.len() < 2 || archive.len() > MAX_FILES + 3 {
        return Err(invalid("Invalid project bundle entry count"));
    }
    let mut entries = HashMap::new();
    let mut total = 0u64;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(zip_error)?;
        let name = entry.name().to_owned();
        if entry.is_dir() || entry.encrypted() || entries.contains_key(&name) {
            return Err(invalid("Invalid or duplicate project bundle entry"));
        }
        let limit = if name == "manifest.json" {
            4096
        } else if name == "project.json" {
            MAX_PROJECT_BYTES
        } else if name == "history.json" {
            history::MAX_HISTORY_BYTES
        } else if let Some(hash) = name.strip_prefix("attachments/") {
            if !crate::artifacts::valid_digest(hash) {
                return Err(invalid("Unsafe project bundle entry"));
            }
            crate::artifacts::MAX_FILE_BYTES as u64
        } else {
            return Err(invalid("Unexpected project bundle entry"));
        };
        if entry.size() > limit {
            return Err(invalid("Project bundle entry is too large"));
        }
        let bytes = read_limited(&mut entry, limit)?;
        total += bytes.len() as u64;
        if total > MAX_ARCHIVE_BYTES {
            return Err(invalid("Project bundle expands beyond the size limit"));
        }
        entries.insert(name, bytes);
    }
    let manifest: Manifest = serde_json::from_slice(
        entries
            .remove("manifest.json")
            .ok_or_else(|| invalid("Project bundle manifest is missing"))?
            .as_slice(),
    )?;
    if manifest.format != "Forma CAD project" || ![1, 2].contains(&manifest.format_version) {
        return Err(invalid("Unsupported project bundle version"));
    }
    let mut project: Project = serde_json::from_slice(
        entries
            .remove("project.json")
            .ok_or_else(|| invalid("Project bundle document is missing"))?
            .as_slice(),
    )?;
    project.validate()?;
    if manifest.project_id != project.id {
        return Err(invalid("Project bundle identity mismatch"));
    }
    if project.files.iter().any(|file| file.data.is_some()) {
        return Err(invalid(
            "Project bundle must store attachments as separate files",
        ));
    }
    crate::artifacts::normalize(&mut project)?;
    let model_history = entries.remove("history.json");
    history::check(
        &project,
        manifest.format_version,
        manifest.history.as_ref(),
        model_history.as_deref(),
    )?;
    let mut pending = Vec::new();
    let mut expected = std::collections::HashSet::new();
    let mut attachment_total = 0u64;
    for file in &project.files {
        let hash = file
            .sha256
            .as_ref()
            .ok_or_else(|| invalid("Project attachment checksum is missing"))?;
        expected.insert(attachment_path(hash));
        let bytes = entries
            .get(&attachment_path(hash))
            .ok_or_else(|| invalid("Project bundle attachment is missing"))?;
        if bytes.len() as u64 != file.size || crate::artifacts::digest(bytes) != *hash {
            return Err(invalid("Project bundle attachment failed integrity check"));
        }
        attachment_total += file.size;
        if attachment_total > 100 * 1024 * 1024 {
            return Err(invalid("Project attachments exceed 100 MB"));
        }
        if !pending.iter().any(|(saved, _)| saved == hash) {
            pending.push((hash.clone(), bytes.clone()));
        }
    }
    if entries.len() != expected.len() {
        return Err(invalid("Project bundle contains unreferenced files"));
    }
    Ok((project, pending, model_history))
}

fn picked_path(value: tauri_plugin_dialog::FilePath) -> Result<PathBuf> {
    let path = value
        .into_path()
        .map_err(|_| invalid("Only local project bundles are supported"))?;
    if !path.is_absolute()
        || path.parent().is_none_or(|parent| !parent.is_dir())
        || std::fs::symlink_metadata(&path).is_ok_and(|meta| meta.file_type().is_symlink())
    {
        return Err(invalid("Unsafe project bundle path"));
    }
    Ok(path)
}

#[tauri::command]
pub async fn import_project_bundle(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<Option<Project>> {
    let source = tokio::task::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("Forma CAD — .cadpack")
            .add_filter("Forma CAD", &["cadpack"])
            .blocking_pick_file()
    })
    .await
    .map_err(|error| invalid(error.to_string()))?;
    let Some(source) = source else {
        return Ok(None);
    };
    let path = picked_path(source)?;
    if path
        .extension()
        .is_none_or(|ext| !ext.eq_ignore_ascii_case("cadpack"))
    {
        return Err(invalid("Expected a .cadpack project bundle"));
    }
    let (mut project, pending, model_history) =
        tokio::task::spawn_blocking(move || read_bundle(&path))
            .await
            .map_err(|error| invalid(error.to_string()))??;
    let _guard = state.writes.lock().await;
    project.id = uuid::Uuid::new_v4().to_string();
    project.created_at = chrono::Utc::now().to_rfc3339();
    project.updated_at = project.created_at.clone();
    let id = project.id.clone();
    let root = crate::security::guarded(&state.root, Path::new(&id))?;
    let result =
        crate::projects::persist_imported_project(&state, project, pending, None, model_history)
            .await;
    // A failure after SQL commit must preserve the committed recovery generation.
    let committed: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM projects WHERE id=?")
        .bind(&id)
        .fetch_one(&state.pool)
        .await?;
    if result.is_err() && committed == 0 && root.exists() {
        let _ = std::fs::remove_dir_all(root);
    }
    result.map(Some)
}

#[cfg(test)]
mod privacy_tests;
#[cfg(test)]
mod tests;
