//! Bounded, self-contained committed snapshots. Legacy DB/blob storage is never removed.
pub mod ipc;
mod locking;
mod manifest;
mod recovery;
pub use recovery::{reconcile, record_pending};
#[cfg(test)]
mod tests;

use crate::{
    core::{AppError, AppState, Result},
    models::Project,
};
pub use locking::ProjectLock;
pub use manifest::Manifest;
use manifest::{Entry, MAX_METADATA, STORAGE_VERSION};
use std::{
    io::{Read, Write},
    path::Path,
};

pub struct StagedSnapshot {
    pub manifest: Manifest,
}

fn bounded_read(root: &Path, path: &Path, maximum: u64) -> Result<Vec<u8>> {
    let guarded = crate::security::guarded(root, path)?;
    let file = std::fs::File::open(guarded)?;
    if file.metadata()?.len() > maximum {
        return Err(AppError::Invalid(
            "Project storage file exceeds its limit".into(),
        ));
    }
    let mut bytes = Vec::new();
    file.take(maximum + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > maximum {
        return Err(AppError::Invalid(
            "Project storage file exceeds its limit".into(),
        ));
    }
    Ok(bytes)
}

fn manifest_bytes(root: &Path, relative: &str) -> Result<Manifest> {
    let manifest: Manifest =
        serde_json::from_slice(&bounded_read(root, Path::new(relative), 2 * 1024 * 1024)?)?;
    manifest.validate()?;
    Ok(manifest)
}

fn read_generation(root: &Path, manifest: &Manifest) -> Result<Project> {
    manifest.validate()?;
    let mut project = None;
    let mut hashes = std::collections::HashMap::new();
    for entry in &manifest.files {
        let relative = entry_relative(manifest, &entry.path);
        let bytes = bounded_read(root, Path::new(&relative), entry.size)?;
        if bytes.len() as u64 != entry.size || crate::artifacts::digest(&bytes) != entry.sha256 {
            return Err(AppError::Invalid(
                "Project storage integrity check failed".into(),
            ));
        }
        if entry.path == "project.json" {
            project = Some(serde_json::from_slice::<Project>(&bytes)?);
        } else if entry.path.starts_with("attachments/") {
            hashes.insert(
                entry.path.trim_start_matches("attachments/").to_string(),
                entry.size,
            );
        }
    }
    let project =
        project.ok_or_else(|| AppError::Invalid("Project storage metadata is missing".into()))?;
    project.validate()?;
    if project.id != manifest.project_id
        || project.files.iter().any(|file| {
            file.data.is_some()
                || file
                    .sha256
                    .as_ref()
                    .is_none_or(|hash| hashes.get(hash) != Some(&file.size))
        })
    {
        return Err(AppError::Invalid(
            "Project storage attachment references are invalid".into(),
        ));
    }
    Ok(project)
}

/// Pure read; portable snapshots do not need the original application's SQLite or .blobs.
pub fn read_folder(root: &Path) -> Result<Project> {
    validate_root(root)?;
    read_generation(root, &manifest_bytes(root, "manifest.json")?)
}

/// Recovery is explicit: callers must present this older state instead of silently overwriting current data.
pub fn read_backup(root: &Path) -> Result<Project> {
    validate_root(root)?;
    read_generation(
        root,
        &manifest_bytes(root, "metadata/manifest.previous.json")?,
    )
}

pub fn hydrate_folder(root: &Path) -> Result<Project> {
    Ok(hydrate_with_history(root)?.0)
}

/// Capture one manifest for both model and undo state, even while another process publishes a new one.
pub fn hydrate_with_history(root: &Path) -> Result<(Project, Option<Vec<u8>>)> {
    hydrate_checked(root, None)
}

/// Inspect and hash the same immutable generation used by the folder review.
pub fn inspect_folder(root: &Path) -> Result<(Project, String)> {
    validate_root(root)?;
    let bytes = bounded_read(root, Path::new("manifest.json"), 2 * 1024 * 1024)?;
    let manifest: Manifest = serde_json::from_slice(&bytes)?;
    Ok((
        read_generation(root, &manifest)?,
        crate::artifacts::digest(&bytes),
    ))
}

pub fn hydrate_checked(root: &Path, expected: Option<&str>) -> Result<(Project, Option<Vec<u8>>)> {
    validate_root(root)?;
    let bytes = bounded_read(root, Path::new("manifest.json"), 2 * 1024 * 1024)?;
    if expected.is_some_and(|hash| {
        !crate::artifacts::valid_digest(hash) || crate::artifacts::digest(&bytes) != hash
    }) {
        return Err(AppError::Invalid("PROJECT_FOLDER_CHANGED".into()));
    }
    let manifest: Manifest = serde_json::from_slice(&bytes)?;
    let mut project = read_generation(root, &manifest)?;
    for file in &mut project.files {
        let relative = format!("assets/{}", file.sha256.as_ref().unwrap());
        let bytes = bounded_read(root, Path::new(&relative), file.size)?;
        if bytes.len() as u64 != file.size
            || crate::artifacts::digest(&bytes) != *file.sha256.as_ref().unwrap()
        {
            return Err(AppError::Invalid(
                "Project storage integrity check failed".into(),
            ));
        }
        file.data = Some(crate::artifacts::data_url(&file.name, &bytes));
    }
    let history = history_for_manifest(root, &manifest)?;
    Ok((project, history))
}

/// Write immutable generation first. Publication remains separate from SQLite commit.
pub fn stage_with(
    root: &Path,
    project: &Project,
    mut read: impl FnMut(&crate::models::ProjectFile) -> Result<Vec<u8>>,
) -> Result<StagedSnapshot> {
    project.validate()?;
    let raw = serde_json::to_vec(project)?;
    if raw.len() as u64 > MAX_METADATA {
        return Err(AppError::Invalid(
            "Project storage metadata exceeds its limit".into(),
        ));
    }
    let generation = uuid::Uuid::new_v4().to_string();
    let prefix = format!("metadata/generations/{generation}");
    let mut files = vec![Entry {
        path: "project.json".into(),
        size: raw.len() as u64,
        sha256: crate::artifacts::digest(&raw),
    }];
    crate::artifacts::immutable_write(root, Path::new(&format!("{prefix}/project.json")), &raw)?;
    let mut included = std::collections::HashSet::new();
    for file in &project.files {
        let hash = file
            .sha256
            .as_ref()
            .filter(|value| crate::artifacts::valid_digest(value))
            .ok_or_else(|| AppError::Invalid("Missing attachment checksum".into()))?;
        if !included.insert(hash.clone()) {
            continue;
        }
        let asset = format!("assets/{hash}");
        let exists = crate::security::guarded(root, Path::new(&asset))?.exists();
        let bytes = if exists {
            bounded_read(root, Path::new(&asset), file.size)?
        } else {
            read(file)?
        };
        if bytes.len() as u64 != file.size || crate::artifacts::digest(&bytes) != *hash {
            return Err(AppError::Invalid(
                "Project storage attachment integrity check failed".into(),
            ));
        }
        let path = format!("attachments/{hash}");
        if !exists {
            crate::artifacts::immutable_write(root, Path::new(&asset), &bytes)?;
        }
        files.push(Entry {
            path,
            size: file.size,
            sha256: hash.clone(),
        });
    }
    let manifest = Manifest {
        storage_version: STORAGE_VERSION,
        project_id: project.id.clone(),
        generation,
        created_at: chrono::Utc::now().to_rfc3339(),
        files,
    };
    read_generation(root, &manifest)?;
    Ok(StagedSnapshot { manifest })
}

pub fn stage(state: &AppState, root: &Path, project: &Project) -> Result<StagedSnapshot> {
    stage_with(root, project, |file| {
        crate::artifacts::read(state, &project.id, file)
    })
}

fn entry_relative(manifest: &Manifest, path: &str) -> String {
    if let Some(hash) = path.strip_prefix("attachments/") {
        format!("assets/{hash}")
    } else {
        format!("metadata/generations/{}/{path}", manifest.generation)
    }
}

pub fn stage_history(root: &Path, staged: &mut StagedSnapshot, bytes: &[u8]) -> Result<()> {
    if bytes.len() > 16 * 1024 * 1024 {
        return Err(AppError::Invalid(
            "Project command history exceeds its limit".into(),
        ));
    }
    let path = format!(
        "metadata/generations/{}/history.json",
        staged.manifest.generation
    );
    crate::artifacts::immutable_write(root, Path::new(&path), bytes)?;
    staged.manifest.files.push(Entry {
        path: "history.json".into(),
        size: bytes.len() as u64,
        sha256: crate::artifacts::digest(bytes),
    });
    staged.manifest.validate()
}

fn validate_root(root: &Path) -> Result<()> {
    if !root.is_absolute() || !root.is_dir() {
        return Err(AppError::Invalid(
            "Project folder must be an existing absolute directory".into(),
        ));
    }
    for ancestor in root.ancestors() {
        if std::fs::symlink_metadata(ancestor)?
            .file_type()
            .is_symlink()
        {
            return Err(AppError::Invalid(
                "Symbolic links are not allowed in project folder paths".into(),
            ));
        }
    }
    Ok(())
}

pub fn read_history(root: &Path) -> Result<Option<Vec<u8>>> {
    validate_root(root)?;
    let manifest = manifest_bytes(root, "manifest.json")?;
    read_generation(root, &manifest)?;
    history_for_manifest(root, &manifest)
}

fn history_for_manifest(root: &Path, manifest: &Manifest) -> Result<Option<Vec<u8>>> {
    manifest
        .files
        .iter()
        .find(|entry| entry.path == "history.json")
        .map(|entry| {
            let bytes = bounded_read(
                root,
                Path::new(&format!(
                    "metadata/generations/{}/history.json",
                    manifest.generation
                )),
                entry.size,
            )?;
            if bytes.len() as u64 != entry.size || crate::artifacts::digest(&bytes) != entry.sha256
            {
                return Err(AppError::Invalid(
                    "Project storage integrity check failed".into(),
                ));
            }
            Ok(bytes)
        })
        .transpose()
}

fn atomic_write(root: &Path, target: &str, bytes: &[u8]) -> Result<()> {
    let temp_name = format!("metadata/commit-{}.tmp", uuid::Uuid::new_v4());
    let temp = crate::security::guarded(root, Path::new(&temp_name))?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    std::fs::rename(temp, crate::security::guarded(root, Path::new(target))?)?;
    Ok(())
}

pub fn publish(root: &Path, staged: StagedSnapshot) -> Result<()> {
    read_generation(root, &staged.manifest)?;
    let old = crate::security::guarded(root, Path::new("manifest.json"))?;
    if old.exists() {
        let previous = manifest_bytes(root, "manifest.json")?;
        read_generation(root, &previous)?;
        atomic_write(
            root,
            "metadata/manifest.previous.json",
            &serde_json::to_vec(&previous)?,
        )?;
    }
    atomic_write(
        root,
        "manifest.json",
        &serde_json::to_vec_pretty(&staged.manifest)?,
    )
}
