//! Immutable CAS publication before SQL references become visible. Errors leave
//! unreferenced new entries for explicit recovery; existing entries are never
//! overwritten or removed. This is not an atomic filesystem/SQL transaction.
use super::read_only::ReadOnlyDatabase;
use crate::{
    core::{AppError, Result},
    models::Project,
    project_access::folder_lease::{FolderDirectory, FolderLease},
};
use std::{collections::HashMap, fs, path::Path};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as platform;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows as platform;

const MAX_BATCH_BYTES: u64 = 100 * 1024 * 1024;

pub(super) struct PreparedCasBatch<'a> {
    references: HashMap<&'a str, u64>,
    pending: HashMap<&'a str, &'a [u8]>,
}

/// Retain until the SQL outcome has been resolved. Closing newly created raw
/// files is safe on Unix: create_new never opened an existing database alias.
/// Reused CAS files are read only through SQLite's own inode bookkeeping.
pub(super) struct PublishedCasBatch {
    directory: FolderDirectory,
    new_files: Vec<platform::NewFile>,
    references: Vec<(String, u64)>,
}

fn invalid() -> AppError {
    AppError::Invalid("PROJECT_FOLDER_ASSET_INVALID".into())
}
fn changed() -> AppError {
    AppError::Invalid("PROJECT_SNAPSHOT_CHANGED".into())
}

impl<'a> PreparedCasBatch<'a> {
    /// Validate the entire candidate and pending batch before any IO. Duplicate
    /// pending hashes are allowed only when their bytes match. Unused pending
    /// entries are rejected rather than published as unexplained garbage.
    pub(super) fn validate(
        candidate: &'a Project,
        pending: &'a [(String, Vec<u8>)],
    ) -> Result<Self> {
        candidate.validate()?;
        let mut references = HashMap::new();
        let mut total = 0_u64;
        for file in &candidate.files {
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
            total = total
                .checked_add(file.size)
                .filter(|total| *total <= MAX_BATCH_BYTES)
                .ok_or_else(invalid)?;
            if references
                .insert(hash, file.size)
                .is_some_and(|size| size != file.size)
            {
                return Err(invalid());
            }
        }
        let mut assets: HashMap<&str, &[u8]> = HashMap::new();
        let mut pending_total = 0_u64;
        for (hash, bytes) in pending {
            pending_total = pending_total
                .checked_add(bytes.len() as u64)
                .filter(|total| *total <= MAX_BATCH_BYTES)
                .ok_or_else(invalid)?;
            if bytes.is_empty()
                || bytes.len() > crate::artifacts::MAX_FILE_BYTES
                || !crate::artifacts::valid_digest(hash)
                || crate::artifacts::digest(bytes) != *hash
                || references.get(hash.as_str()) != Some(&(bytes.len() as u64))
            {
                return Err(invalid());
            }
            if assets
                .insert(hash, bytes)
                .is_some_and(|previous| previous != bytes)
            {
                return Err(invalid());
            }
        }
        Ok(Self {
            references,
            pending: assets,
        })
    }

    /// Bootstrap mode: no database reader exists yet. Every reference must be
    /// supplied, and an existing digest is refused without opening it. The caller
    /// owns the separate empty-folder/database-create protocol.
    pub(super) fn publish_new(self, lease: &FolderLease) -> Result<PublishedCasBatch> {
        if self
            .references
            .keys()
            .any(|hash| !self.pending.contains_key(hash))
        {
            return Err(invalid());
        }
        lease.recheck()?;
        let directory = lease.assets_directory()?;
        // A bootstrap collision is refused before creating any batch entry.
        for hash in self.references.keys() {
            match fs::symlink_metadata(directory.path().join(hash)) {
                Ok(_) => return Err(invalid()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
                Err(error) => return Err(error.into()),
            }
        }
        let mut new_files = Vec::with_capacity(self.pending.len());
        for (hash, bytes) in self.pending {
            directory.recheck()?;
            let mut file = platform::NewFile::create(directory.path(), hash)?;
            file.write(bytes)?;
            file.recheck()?;
            new_files.push(file);
        }
        platform::sync_directory(directory.path())?;
        let published = PublishedCasBatch {
            directory,
            new_files,
            references: self
                .references
                .into_iter()
                .map(|(hash, size)| (hash.into(), size))
                .collect(),
        };
        published.recheck_new()?;
        Ok(published)
    }

    /// Synchronous by design. Async callers must move the reader and retained
    /// guards into their blocking job, then await its actual completion before
    /// SQL publication; cancelling the caller must not revoke a running job.
    pub(super) fn publish(
        self,
        lease: &FolderLease,
        existing: &mut ReadOnlyDatabase,
    ) -> Result<PublishedCasBatch> {
        lease.recheck()?;
        existing.recheck_location()?;
        let directory = lease.assets_directory()?;
        let mut missing = Vec::new();
        // Reject any missing/corrupt reused reference before creating new blobs.
        for (&hash, &size) in &self.references {
            match fs::symlink_metadata(directory.path().join(hash)) {
                Ok(_) => {
                    existing.read_blob(hash, size)?;
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    let bytes = self.pending.get(hash).copied().ok_or_else(invalid)?;
                    missing.push((hash, bytes));
                }
                Err(error) => return Err(error.into()),
            }
        }
        directory.recheck()?;
        existing.recheck_location()?;
        let mut new_files = Vec::with_capacity(missing.len());
        for (hash, bytes) in missing {
            directory.recheck()?;
            match platform::NewFile::create(directory.path(), hash) {
                Ok(mut file) => {
                    file.write(bytes)?;
                    file.recheck()?;
                    new_files.push(file);
                }
                Err(AppError::Io(error)) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    // A concurrent entry is never truncated or opened raw.
                    existing.read_blob(hash, bytes.len() as u64)?;
                }
                Err(error) => return Err(error),
            }
        }
        platform::sync_directory(directory.path())?;
        let published = PublishedCasBatch {
            directory,
            new_files,
            references: self
                .references
                .into_iter()
                .map(|(hash, size)| (hash.into(), size))
                .collect(),
        };
        published.recheck(existing)?;
        Ok(published)
    }
}

impl PublishedCasBatch {
    /// Cheap guards for each SQLite IO operation. Full content validation is
    /// performed at publication boundaries, not once per database page.
    pub(super) fn recheck_identity(&self) -> Result<()> {
        self.directory.recheck()?;
        for file in &self.new_files {
            file.recheck_identity()?;
        }
        self.directory.recheck()
    }
    pub(super) fn recheck_new(&self) -> Result<()> {
        self.directory.recheck()?;
        for file in &self.new_files {
            file.recheck()?;
        }
        self.directory.recheck()
    }
    /// This recheck detects observed replacement/corruption; it cannot lock out
    /// unrelated filesystem writers. SQL must still compare its original state
    /// inside its own transaction after this check.
    pub(super) fn recheck(&self, existing: &mut ReadOnlyDatabase) -> Result<()> {
        self.recheck_new()?;
        existing.recheck_location()?;
        for (hash, size) in &self.references {
            existing.read_blob(hash, *size)?;
        }
        self.directory.recheck()?;
        existing.recheck_location()
    }
}

fn ordinary(path: &Path) -> Result<fs::Metadata> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(changed());
    }
    Ok(metadata)
}

#[cfg(test)]
mod tests;
