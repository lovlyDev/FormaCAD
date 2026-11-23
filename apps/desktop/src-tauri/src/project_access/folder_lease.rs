//! Cooperative writer authority for one external source location.
//! Never reconstructed from project IDs, UI tokens or serialized state. Clones
//! retain the same logical lease and path handles until the last job finishes.
use crate::core::{AppError, Result};
use std::{
    fs::File,
    path::{Path, PathBuf},
    sync::Arc,
};

#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
#[path = "folder_lease/platform_linux.rs"]
mod platform;
#[cfg(windows)]
#[path = "folder_lease/platform_windows.rs"]
mod platform;
#[cfg(not(any(
    windows,
    all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )
)))]
#[path = "folder_lease/platform_unsupported.rs"]
mod platform;

#[derive(Clone)]
pub(crate) struct FolderLease(Arc<Inner>);

/// Retained CAS directory authority. The caller must reject unsafe leaf names,
/// links and unexpected existing files; this pins the directory, not its leaves.
pub(crate) struct FolderDirectory {
    lease: FolderLease,
    directory: platform::AssetDirectory,
}

struct Inner {
    // The explicit Drop runs while all source path handles are still retained.
    lock: File,
    source: platform::Source,
}

impl FolderLease {
    pub(crate) fn acquire(root: &Path) -> Result<Self> {
        let source = platform::Source::capture(root).map_err(access_error)?;
        Self::from_source(source)
    }

    /// Initialize inside the user's explicitly selected, pre-existing empty
    /// directory. Its first captured identity remains authoritative throughout.
    /// Does not claim to create the physical root, or reserve its whole namespace
    /// against unrelated actors creating files concurrently. The database writer
    /// must separately use create-new semantics and never overwrite an old DB.
    pub(crate) fn initialize_empty(root: &Path) -> Result<Self> {
        let source = platform::Source::initialize_empty(root).map_err(access_error)?;
        Self::from_source(source)
    }

    fn from_source(source: platform::Source) -> Result<Self> {
        let lock = source.open_lock().map_err(access_error)?;
        match lock.try_lock() {
            Ok(()) => (),
            Err(std::fs::TryLockError::WouldBlock) => return Err(read_only()),
            Err(std::fs::TryLockError::Error(_)) => return Err(unavailable()),
        }
        // Wrap before any fallible post-acquisition checks, including rechecks.
        let lease = Self(Arc::new(Inner { lock, source }));
        lease.recheck()?;
        Ok(lease)
    }

    pub(crate) fn recheck(&self) -> Result<()> {
        self.0.source.recheck(&self.0.lock).map_err(|_| changed())
    }

    /// Private process-local SQLite path. Never serialize it. A lease clone MUST
    /// survive until SQLx has asynchronously closed every associated connection.
    /// SQLite integration must reject links at the database leaf and verify its
    /// actual opened identity; this method never opens an independent DB fd.
    pub(crate) fn database_path(&self) -> PathBuf {
        self.0.source.database_path()
    }

    pub(crate) fn assets_directory(&self) -> Result<FolderDirectory> {
        self.recheck()?;
        let directory = self.0.source.assets_directory().map_err(access_error)?;
        let guard = FolderDirectory {
            lease: self.clone(),
            directory,
        };
        guard.recheck()?;
        Ok(guard)
    }
}

impl FolderDirectory {
    /// Process-local pinned path; retain this guard through all asynchronous IO.
    pub(crate) fn path(&self) -> &Path {
        self.directory.path()
    }

    pub(crate) fn recheck(&self) -> Result<()> {
        self.lease.recheck()?;
        self.directory
            .recheck(&self.lease.0.source)
            .map_err(|_| changed())
    }
}

impl Drop for Inner {
    fn drop(&mut self) {
        if let Err(error) = self.lock.unlock() {
            tracing::warn!(%error, "Folder writer lease unlock failed");
        }
    }
}

fn changed() -> AppError {
    AppError::Invalid("PROJECT_SNAPSHOT_CHANGED".into())
}

fn access_error(error: AppError) -> AppError {
    match error {
        AppError::Io(_) => unavailable(),
        other => other,
    }
}

fn unavailable() -> AppError {
    AppError::Invalid("PROJECT_ACCESS_UNAVAILABLE".into())
}

fn read_only() -> AppError {
    AppError::Invalid("PROJECT_READ_ONLY".into())
}

#[cfg(test)]
#[path = "folder_lease/tests.rs"]
mod tests;
