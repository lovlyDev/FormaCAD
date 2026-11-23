use std::{
    fs::{File, Metadata, OpenOptions},
    io,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
};

pub(super) struct Directory {
    // Retaining the inode prevents it being recycled for a replacement directory.
    _handle: File,
    device: u64,
    inode: u64,
}

impl Directory {
    pub(super) fn open(path: &Path) -> io::Result<Self> {
        // Reject non-directories inside open(2), before obtaining a descriptor.
        // Opening and then closing an SQLite file just to inspect its metadata
        // can release this process's unrelated POSIX record locks on that file.
        let handle = OpenOptions::new()
            .read(true)
            .custom_flags(directory_flags()?)
            .open(path)?;
        let metadata = handle.metadata()?;
        let path_metadata = std::fs::symlink_metadata(path)?;
        if !metadata.is_dir()
            || is_link(&path_metadata)
            || metadata.dev() != path_metadata.dev()
            || metadata.ino() != path_metadata.ino()
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Directory identity changed",
            ));
        }
        Ok(Self {
            _handle: handle,
            device: metadata.dev(),
            inode: metadata.ino(),
        })
    }

    pub(super) fn same(&self, other: &Self) -> bool {
        self.device == other.device && self.inode == other.inode
    }
}

pub(super) fn is_link(metadata: &Metadata) -> bool {
    metadata.file_type().is_symlink()
}

// Linux asm-generic/fcntl.h and Apple XNU bsd/sys/fcntl.h. Do not guess flags
// for other Unix targets; their directory identity support fails closed.
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
fn directory_flags() -> io::Result<i32> {
    Ok((1 << 16) | (1 << 17))
}

#[cfg(target_os = "macos")]
fn directory_flags() -> io::Result<i32> {
    Ok(0x0010_0000 | 0x0000_0100)
}

#[cfg(not(any(
    target_os = "macos",
    all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )
)))]
fn directory_flags() -> io::Result<i32> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "PROJECT_ACCESS_UNAVAILABLE",
    ))
}

#[cfg(all(test, any(target_os = "linux", target_os = "macos")))]
#[path = "unix/tests.rs"]
mod tests;
