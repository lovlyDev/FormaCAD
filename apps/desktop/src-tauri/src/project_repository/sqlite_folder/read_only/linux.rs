//! A retained directory alias binds Unix xOpen without opening a raw database FD.
//! SQLite's Unix xOpen uses O_NOFOLLOW on the final component. The directory
//! component is our retained procfs alias, not the caller's raceable ancestors.
use super::{abi, invalid};
use crate::core::Result;
use std::{
    ffi::CString,
    fs::{self, File, Metadata, OpenOptions},
    os::{
        fd::AsRawFd,
        unix::fs::{MetadataExt, OpenOptionsExt},
    },
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::project_repository::sqlite_folder) struct FileIdentity {
    device: u64,
    inode: u64,
}
pub(in crate::project_repository::sqlite_folder) struct Binding {
    directory: File,
    identity: FileIdentity,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::project_repository::sqlite_folder) struct FileRecord {
    pub identity: FileIdentity,
    pub size: u64,
}

pub(in crate::project_repository::sqlite_folder) fn path_identity(
    path: &Path,
) -> Result<FileIdentity> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.nlink() != 1 {
        return Err(invalid());
    }
    Ok(identity(&metadata))
}
fn identity(metadata: &Metadata) -> FileIdentity {
    FileIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
    }
}

fn directory(path: &Path) -> Result<File> {
    // These flags are audited for Linux x86_64/aarch64, the supported desktop
    // targets. O_DIRECTORY rejects regular-file races before an FD is opened;
    // closing a wrongly opened database FD would disrupt POSIX SQLite locks.
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    {
        Ok(OpenOptions::new()
            .read(true)
            .custom_flags(0x1_0000 | 0x2_0000)
            .open(path)?)
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        let _ = path;
        Err(super::unsupported())
    }
}

impl Binding {
    pub(in crate::project_repository::sqlite_folder) fn capture(root: &Path) -> Result<Self> {
        let directory = directory(root)?;
        let actual = directory.metadata()?;
        let current = fs::symlink_metadata(root)?;
        if !actual.is_dir()
            || !current.is_dir()
            || current.file_type().is_symlink()
            || identity(&actual) != identity(&current)
        {
            return Err(invalid());
        }
        // Verify procfs supports this retained directory before any SQLite call.
        let alias = PathBuf::from(format!("/proc/self/fd/{}", directory.as_raw_fd()));
        if identity(&fs::metadata(alias)?) != identity(&actual) {
            return Err(invalid());
        }
        Ok(Self {
            directory,
            identity: identity(&actual),
        })
    }
    pub(in crate::project_repository::sqlite_folder) fn path(&self, name: &str) -> PathBuf {
        PathBuf::from(format!(
            "/proc/self/fd/{}/{}",
            self.directory.as_raw_fd(),
            name
        ))
    }
    pub(in crate::project_repository::sqlite_folder) fn alias(
        &self,
        name: &str,
    ) -> Result<CString> {
        CString::new(self.path(name).as_os_str().as_encoded_bytes()).map_err(|_| invalid())
    }

    pub(in crate::project_repository::sqlite_folder) fn subdirectory(
        &self,
        name: &str,
    ) -> Result<Self> {
        Self::capture(&self.path(name))
    }
    pub(in crate::project_repository::sqlite_folder) fn matches_directory(
        &self,
        path: &Path,
    ) -> Result<bool> {
        let metadata = fs::symlink_metadata(path)?;
        Ok(metadata.is_dir()
            && !metadata.file_type().is_symlink()
            && identity(&metadata) == self.identity)
    }
    pub(in crate::project_repository::sqlite_folder) fn file_record(
        &self,
        name: &str,
    ) -> Result<FileRecord> {
        let metadata = fs::symlink_metadata(self.path(name))?;
        if !metadata.is_file()
            || metadata.file_type().is_symlink()
            || metadata.nlink() != 1
            || metadata.dev() != self.identity.device
        {
            return Err(invalid());
        }
        Ok(FileRecord {
            identity: identity(&metadata),
            size: metadata.len(),
        })
    }

    /// SAFETY: inner is the live base SQLite file for this binding, protected
    /// against concurrent SQLite access by xOpen or lock_handle.
    pub(in crate::project_repository::sqlite_folder) unsafe fn actual_identity(
        &self,
        inner: *mut abi::File,
        name: &str,
    ) -> Result<FileIdentity> {
        let current = path_identity(&self.path(name))?;
        if current.device != self.identity.device {
            return Err(invalid());
        }
        let mut moved = 1_i32;
        // SAFETY: the base's public method table belongs to this live file.
        let method = unsafe { (*(*inner).pMethods).xFileControl }.ok_or_else(invalid)?;
        // HAS_MOVED must be implemented and explicitly false, not NOTFOUND.
        let rc = unsafe { method(inner, abi::HAS_MOVED, (&mut moved as *mut i32).cast()) };
        if rc != abi::OK || moved != 0 {
            return Err(invalid());
        }
        Ok(current)
    }
    pub(in crate::project_repository::sqlite_folder) unsafe fn actual_file_record(
        &self,
        inner: *mut abi::File,
        name: &str,
    ) -> Result<FileRecord> {
        let actual = unsafe { self.actual_identity(inner, name) }?;
        let record = self.file_record(name)?;
        if record.identity != actual {
            return Err(invalid());
        }
        Ok(record)
    }
}
