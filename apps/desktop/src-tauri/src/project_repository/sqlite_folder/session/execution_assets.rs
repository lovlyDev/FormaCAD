//! Async verified reads feed the existing synchronous CAD asset callback.
//! Cached bytes are inputs, never authority to publish a worker result.
use super::{changed, FolderSession, FolderSnapshot, ReadBinding};
use crate::{
    core::{AppError, Result},
    modeling::execution::AssetSource,
    models::{Project, ProjectFile},
};
use std::collections::HashMap;

const MAX_PREFETCH_BYTES: u64 = 100 * 1024 * 1024;

pub(in crate::project_repository::sqlite_folder) struct ExecutionAssets<'a> {
    binding: ReadBinding,
    descriptors: Vec<&'a ProjectFile>,
    bytes: HashMap<&'a str, Vec<u8>>,
}

fn invalid() -> AppError {
    AppError::Invalid("PROJECT_FOLDER_ASSET_INVALID".into())
}

// Complete the bounded metadata pass before allocating any CAS payload buffers.
// The host supplies a descriptor subset, not one descriptor per document use.
// Repeated references may share bytes but cannot give an unrequested name access.
fn select<'a>(project: &'a Project, requested: &[ProjectFile]) -> Result<Vec<&'a ProjectFile>> {
    if requested.len() > project.files.len() {
        return Err(invalid());
    }
    let saved: HashMap<_, _> = project
        .files
        .iter()
        .map(|file| (file.name.as_str(), file))
        .collect();
    let mut sizes = HashMap::new();
    let mut total = 0_u64;
    for file in requested {
        if saved.get(file.name.as_str()).copied() != Some(file) {
            return Err(changed());
        }
        let hash = file
            .sha256
            .as_deref()
            .filter(|hash| crate::artifacts::valid_digest(hash))
            .ok_or_else(invalid)?;
        if file.data.is_some()
            || file.size == 0
            || file.size > crate::artifacts::MAX_FILE_BYTES as u64
        {
            return Err(invalid());
        }
        if let Some(previous) = sizes.insert(hash, file.size) {
            if previous != file.size {
                return Err(invalid());
            }
        } else {
            total = total
                .checked_add(file.size)
                .filter(|sum| *sum <= MAX_PREFETCH_BYTES)
                .ok_or_else(invalid)?;
        }
    }
    Ok(requested
        .iter()
        .map(|file| saved[file.name.as_str()])
        .collect())
}

impl<'a> ExecutionAssets<'a> {
    pub(in crate::project_repository::sqlite_folder) async fn prefetch(
        session: &FolderSession,
        original: &'a FolderSnapshot,
        requested: &[ProjectFile],
    ) -> Result<Self> {
        session.owns(original)?;
        let descriptors = select(original.project(), requested)?;
        let binding = session.read_binding(original).await?;
        binding.check_active()?;
        let mut bytes = HashMap::new();
        for descriptor in &descriptors {
            binding.check_active()?;
            let hash = descriptor.sha256.as_deref().ok_or_else(invalid)?;
            if let std::collections::hash_map::Entry::Vacant(entry) = bytes.entry(hash) {
                let value = session.read_asset(original, descriptor).await?;
                binding.check_active()?;
                entry.insert(value);
            }
        }
        session.recheck(original).await?;
        binding.check_active()?;
        Ok(Self {
            binding,
            descriptors,
            bytes,
        })
    }
}

impl AssetSource for ExecutionAssets<'_> {
    fn read(&mut self, file: &ProjectFile) -> Result<Vec<u8>> {
        self.binding.check_active()?;
        let descriptor = self
            .descriptors
            .iter()
            .find(|saved| **saved == file)
            .ok_or_else(changed)?;
        let hash = descriptor.sha256.as_deref().ok_or_else(invalid)?;
        let bytes = self.bytes.get(hash).ok_or_else(changed)?.clone();
        self.binding.check_active()?;
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests;
