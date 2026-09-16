use crate::{
    core::{AppError, AppState, Result},
    models::{Project, ProjectFile},
};
use base64::{engine::general_purpose::STANDARD, Engine};
use sha2::{Digest, Sha256};
use std::{
    io::Write,
    path::{Path, PathBuf},
};

pub const MAX_FILE_BYTES: usize = 40 * 1024 * 1024;
pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
pub fn decode_data(data: &str) -> Result<Vec<u8>> {
    if data.len() > MAX_FILE_BYTES * 4 / 3 + 1024 {
        return Err(AppError::Invalid("Attachment exceeds 40 MB".into()));
    }
    let (header, encoded) = data
        .split_once(',')
        .ok_or_else(|| AppError::Invalid("Malformed attachment data".into()))?;
    if !header.starts_with("data:") || !header.ends_with(";base64") {
        return Err(AppError::Invalid(
            "Only base64 file attachments are accepted".into(),
        ));
    }
    let bytes = STANDARD
        .decode(encoded)
        .map_err(|_| AppError::Invalid("Invalid attachment encoding".into()))?;
    if bytes.len() > MAX_FILE_BYTES || bytes.is_empty() {
        return Err(AppError::Invalid("Attachment is empty or too large".into()));
    }
    Ok(bytes)
}
pub fn mime(name: &str) -> &'static str {
    match Path::new(name)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "glb" => "model/gltf-binary",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "pdf" => "application/pdf",
        "stl" => "model/stl",
        _ => "application/octet-stream",
    }
}
pub fn data_url(name: &str, bytes: &[u8]) -> String {
    format!("data:{};base64,{}", mime(name), STANDARD.encode(bytes))
}
fn extension(name: &str) -> Result<String> {
    let ext = Path::new(name)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if ![
        "step", "stp", "stl", "obj", "glb", "3mf", "dxf", "png", "jpg", "jpeg", "webp", "pdf",
    ]
    .contains(&ext.as_str())
    {
        return Err(AppError::Invalid("Unsupported attachment type".into()));
    }
    Ok(ext)
}
pub fn immutable_write(root: &Path, relative: &Path, bytes: &[u8]) -> Result<PathBuf> {
    let target = crate::security::guarded(root, relative)?;
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&target)
    {
        Ok(mut file) => {
            file.write_all(bytes)?;
            file.sync_all()?;
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            let existing = std::fs::read(crate::security::guarded(root, relative)?)?;
            if digest(&existing) != digest(bytes) {
                return Err(AppError::Invalid(
                    "An immutable project artifact was modified outside the application".into(),
                ));
            }
        }
        Err(e) => return Err(e.into()),
    }
    Ok(target)
}
pub fn normalize(project: &mut Project) -> Result<Vec<(String, Vec<u8>)>> {
    let mut pending = Vec::new();
    let mut total = 0u64;
    for file in &mut project.files {
        crate::security::file_name(&file.name)?;
        extension(&file.name)?;
        if let Some(data) = file.data.take() {
            let bytes = decode_data(&data)?;
            let hash = digest(&bytes);
            if file.sha256.as_ref().is_some_and(|h| h != &hash) {
                return Err(AppError::Invalid("Attachment checksum mismatch".into()));
            }
            file.size = bytes.len() as u64;
            file.sha256 = Some(hash.clone());
            pending.push((hash, bytes));
        }
        if file.sha256.as_ref().is_none_or(|h| !valid_digest(h)) {
            return Err(AppError::Invalid("Missing attachment checksum".into()));
        }
        total += file.size;
    }
    if total > 100 * 1024 * 1024 {
        return Err(AppError::Invalid(
            "Project attachments exceed 100 MB".into(),
        ));
    }
    Ok(pending)
}
pub fn materialize(
    state: &AppState,
    project: &Project,
    pending: &[(String, Vec<u8>)],
) -> Result<()> {
    for (hash, bytes) in pending {
        immutable_write(&state.root, Path::new(&format!(".blobs/{hash}")), bytes)?;
    }
    for file in &project.files {
        let hash = file
            .sha256
            .as_ref()
            .ok_or_else(|| AppError::Invalid("Missing attachment checksum".into()))?;
        let blob = crate::security::guarded(&state.root, Path::new(&format!(".blobs/{hash}")))?;
        let bytes = std::fs::read(blob)?;
        if bytes.len() as u64 != file.size {
            return Err(AppError::Invalid(
                "Attachment size does not match stored content".into(),
            ));
        }
        if digest(&bytes) != *hash {
            return Err(AppError::Invalid(
                "Cached artifact checksum mismatch".into(),
            ));
        }
        immutable_write(
            &state.root,
            Path::new(&format!(
                "{}/attachments/{}.{}",
                project.id,
                hash,
                extension(&file.name)?
            )),
            &bytes,
        )?;
    }
    Ok(())
}
pub fn read(state: &AppState, project_id: &str, file: &ProjectFile) -> Result<Vec<u8>> {
    if let Some(data) = &file.data {
        return decode_data(data);
    }
    let hash = file
        .sha256
        .as_ref()
        .filter(|h| valid_digest(h))
        .ok_or_else(|| AppError::Invalid("Artifact checksum is missing".into()))?;
    let path = crate::security::guarded(
        &state.root,
        Path::new(&format!(
            "{project_id}/attachments/{hash}.{}",
            extension(&file.name)?
        )),
    )?;
    let bytes = std::fs::read(path)?;
    if digest(&bytes) != *hash {
        return Err(AppError::Invalid(
            "Project file failed its integrity check".into(),
        ));
    }
    Ok(bytes)
}
#[tauri::command]
pub async fn read_project_file(
    project_id: String,
    name: String,
    state: tauri::State<'_, AppState>,
) -> Result<String> {
    let p = crate::projects::get(&state, &project_id).await?;
    let file = p
        .files
        .iter()
        .find(|f| f.name == name)
        .ok_or_else(|| AppError::Invalid("Project attachment was not found".into()))?;
    Ok(data_url(&name, &read(&state, &project_id, file)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn altered_immutable_file_is_rejected() {
        let d = tempfile::tempdir().unwrap();
        immutable_write(d.path(), Path::new("a/file.stl"), b"first").unwrap();
        assert!(immutable_write(d.path(), Path::new("a/file.stl"), b"second").is_err());
        assert_eq!(
            std::fs::read(d.path().join("a/file.stl")).unwrap(),
            b"first"
        );
    }
    #[test]
    fn invalid_data_and_hash_are_rejected() {
        assert!(decode_data("https://example.com/a.step").is_err());
        assert!(decode_data("data:model/stl;base64,?invalid").is_err());
        assert!(!valid_digest("../escape"));
        assert!(valid_digest(&digest(b"model")));
    }
}
