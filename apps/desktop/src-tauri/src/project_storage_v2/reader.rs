//! Metadata-only reads of a sealed folder generation, with explicit asset/history reads.
//! This seal checks saved bytes, not directory identity or a repository lifecycle.
//! Callers must retain and recheck their original repository/location session too.
use super::{
    bounded_read, entry_relative,
    manifest::{Entry, Manifest},
    validate_root,
};
use crate::{
    core::{AppError, Result},
    models::{Project, ProjectFile},
};
use std::{collections::HashMap, fs, path::Path};

const MAX_MANIFEST: u64 = 2 * 1024 * 1024;

pub struct FolderSnapshot {
    project: Project,
    manifest: Manifest,
    seal: String,
}

impl FolderSnapshot {
    /// Captures manifest and project metadata only. It neither loads nor validates
    /// unrelated asset bytes or the history archive, and never performs recovery.
    pub fn capture(root: &Path) -> Result<Self> {
        validate_root(root)?;
        check_regular(root, Path::new("manifest.json"))?;
        let bytes = bounded_read(root, Path::new("manifest.json"), MAX_MANIFEST)?;
        // Parse the exact bytes being sealed; a second independent manifest read
        // could otherwise attach a new seal to metadata from an older generation.
        let manifest: Manifest = serde_json::from_slice(&bytes)?;
        manifest.validate()?;
        let seal = crate::artifacts::digest(&bytes);
        let entry = manifest
            .files
            .iter()
            .find(|entry| entry.path == "project.json")
            .ok_or_else(changed)?;
        let bytes = read_entry(root, &manifest, entry)?;
        let project: Project = serde_json::from_slice(&bytes)?;
        project.validate()?;
        let attachments: HashMap<_, _> = manifest
            .files
            .iter()
            .filter_map(|entry| {
                entry
                    .path
                    .strip_prefix("attachments/")
                    .map(|hash| (hash, entry))
            })
            .collect();
        if project.id != manifest.project_id
            || project.files.iter().any(|file| {
                file.data.is_some()
                    || file.sha256.as_ref().is_none_or(|hash| {
                        attachments
                            .get(hash.as_str())
                            .is_none_or(|entry| entry.size != file.size)
                    })
            })
        {
            return Err(changed());
        }
        let snapshot = Self {
            project,
            manifest,
            seal,
        };
        snapshot.recheck(root)?;
        Ok(snapshot)
    }

    pub fn project(&self) -> &Project {
        &self.project
    }

    /// Checks the original manifest seal. Immutable-generation metadata was
    /// validated during capture; full history/artifact integrity is explicit.
    pub fn recheck(&self, root: &Path) -> Result<()> {
        validate_root(root)?;
        check_regular(root, Path::new("manifest.json"))?;
        let bytes = bounded_read(root, Path::new("manifest.json"), MAX_MANIFEST)?;
        if crate::artifacts::digest(&bytes) != self.seal {
            return Err(changed());
        }
        // An unchanged manifest does not prove immutable files stayed intact.
        // Metadata is bounded and cheap relative to loading all referenced assets.
        let entry = self
            .manifest
            .files
            .iter()
            .find(|entry| entry.path == "project.json")
            .ok_or_else(changed)?;
        read_entry(root, &self.manifest, entry)?;
        Ok(())
    }

    pub fn read_asset(&self, root: &Path, file: &ProjectFile) -> Result<Vec<u8>> {
        self.recheck(root)?;
        if !self.project.files.contains(file) {
            return Err(changed());
        }
        let hash = file.sha256.as_ref().ok_or_else(changed)?;
        let path = format!("attachments/{hash}");
        let entry = self
            .manifest
            .files
            .iter()
            .find(|entry| entry.path == path)
            .filter(|entry| entry.size == file.size && entry.sha256 == *hash)
            .ok_or_else(changed)?;
        let bytes = read_entry(root, &self.manifest, entry)?;
        self.recheck(root)?;
        Ok(bytes)
    }

    pub fn read_history(&self, root: &Path) -> Result<Option<Vec<u8>>> {
        self.recheck(root)?;
        let bytes = self
            .manifest
            .files
            .iter()
            .find(|entry| entry.path == "history.json")
            .map(|entry| -> Result<Vec<u8>> {
                let bytes = read_entry(root, &self.manifest, entry)?;
                crate::project_history::validate_archive(&self.project, &bytes)?;
                Ok(bytes)
            })
            .transpose()?;
        self.recheck(root)?;
        Ok(bytes)
    }
}

fn read_entry(root: &Path, manifest: &Manifest, entry: &Entry) -> Result<Vec<u8>> {
    let relative = entry_relative(manifest, &entry.path);
    check_regular(root, Path::new(&relative))?;
    // Manifest validation caps each size before allocation; bounded_read also
    // checks current length and reads at most expected+1 if the file grows.
    let bytes = bounded_read(root, Path::new(&relative), entry.size)?;
    if bytes.len() as u64 != entry.size || crate::artifacts::digest(&bytes) != entry.sha256 {
        return Err(changed());
    }
    Ok(bytes)
}

fn check_regular(root: &Path, relative: &Path) -> Result<()> {
    reject_links(root)?;
    let path = crate::security::guarded(root, relative)?;
    reject_links(&path)?;
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() {
        return Err(changed());
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err(changed());
        }
    }
    Ok(())
}

fn reject_links(path: &Path) -> Result<()> {
    for ancestor in path.ancestors() {
        let metadata = fs::symlink_metadata(ancestor)?;
        if metadata.file_type().is_symlink() {
            return Err(changed());
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if metadata.file_attributes() & 0x400 != 0 {
                return Err(changed());
            }
        }
    }
    Ok(())
}

fn changed() -> AppError {
    AppError::Invalid("PROJECT_SNAPSHOT_CHANGED".into())
}

#[cfg(test)]
#[path = "reader_tests.rs"]
mod tests;
