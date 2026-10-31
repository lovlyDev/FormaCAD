//! Metadata-only mirror authority. Asset integrity is checked by asset/action
//! consumers; controls do not promise to audit unrelated CAS/archive contents.
use super::{changed, controls_projection::Projection};
use crate::{
    core::{AppState, Result},
    project_repository::location::LocationIdentity,
    project_storage_v2::Manifest,
};
use std::{
    io::Read,
    path::{Path, PathBuf},
};

pub(super) struct FolderSeal {
    manifest_hash: String,
    metadata_path: PathBuf,
    metadata_size: u64,
    metadata_hash: String,
    location: LocationIdentity,
}

fn read(root: &Path, relative: &Path, maximum: u64) -> Result<Vec<u8>> {
    let path = crate::security::guarded(root, relative)?;
    let metadata = std::fs::symlink_metadata(&path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(changed());
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err(changed());
        }
    }
    let file = std::fs::File::open(path)?;
    if file.metadata()?.len() > maximum {
        return Err(changed());
    }
    let mut bytes = Vec::new();
    file.take(maximum + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > maximum {
        return Err(changed());
    }
    Ok(bytes)
}

impl FolderSeal {
    /// None means unsupported byte representation, requiring the old full reader.
    pub fn capture(
        state: &AppState,
        projection: &Projection,
        payload_hash: &str,
        payload_len: usize,
    ) -> Result<Option<Self>> {
        let root = state.root.join(&projection.id);
        let bytes = read(&root, Path::new("manifest.json"), 2 * 1024 * 1024)?;
        let manifest: Manifest = serde_json::from_slice(&bytes)?;
        manifest.validate()?;
        if manifest.project_id != projection.id {
            return Err(changed());
        }
        let entry = manifest
            .files
            .iter()
            .find(|entry| entry.path == "project.json")
            .ok_or_else(changed)?;
        // Older writers/noncanonical JSON require semantic normalization by the
        // full reader. A hash mismatch is never treated as an eligible fast seal.
        if entry.sha256 != payload_hash || entry.size != payload_len as u64 {
            return Ok(None);
        }
        if projection.files.iter().any(|file| {
            file.sha256.as_ref().is_none_or(|hash| {
                !manifest.files.iter().any(|entry| {
                    entry.path == format!("attachments/{hash}")
                        && entry.sha256 == *hash
                        && entry.size == file.size
                })
            })
        }) {
            return Err(changed());
        }
        let directory = PathBuf::from("metadata/generations").join(&manifest.generation);
        let metadata_path = directory.join("project.json");
        let seal = Self {
            manifest_hash: crate::artifacts::digest(&bytes),
            metadata_path,
            metadata_size: entry.size,
            metadata_hash: entry.sha256.clone(),
            location: LocationIdentity::capture(&root, &directory)?,
        };
        seal.recheck(state, &projection.id)?;
        Ok(Some(seal))
    }

    pub fn recheck(&self, state: &AppState, id: &str) -> Result<()> {
        let root = state.root.join(id);
        let directory = self.metadata_path.parent().ok_or_else(changed)?;
        if !self.location.recheck(&root, directory)? {
            return Err(changed());
        }
        let manifest = read(&root, Path::new("manifest.json"), 2 * 1024 * 1024)?;
        if crate::artifacts::digest(&manifest) != self.manifest_hash {
            return Err(changed());
        }
        let path = crate::security::guarded(&root, &self.metadata_path)?;
        let metadata = std::fs::symlink_metadata(&path)?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err(changed());
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if metadata.file_attributes() & 0x400 != 0 {
                return Err(changed());
            }
        }
        let mut file = std::fs::File::open(path)?;
        if file.metadata()?.len() != self.metadata_size {
            return Err(changed());
        }
        use sha2::{Digest, Sha256};
        let mut hash = Sha256::new();
        let mut total = 0_u64;
        let mut buffer = [0_u8; 32768];
        loop {
            let size = file.read(&mut buffer)?;
            if size == 0 {
                break;
            }
            total = total.checked_add(size as u64).ok_or_else(changed)?;
            if total > self.metadata_size {
                return Err(changed());
            }
            hash.update(&buffer[..size]);
        }
        if total != self.metadata_size || format!("{:x}", hash.finalize()) != self.metadata_hash {
            return Err(changed());
        }
        let latest = read(&root, Path::new("manifest.json"), 2 * 1024 * 1024)?;
        if crate::artifacts::digest(&latest) != self.manifest_hash
            || !self.location.recheck(&root, directory)?
        {
            return Err(changed());
        }
        Ok(())
    }
}
