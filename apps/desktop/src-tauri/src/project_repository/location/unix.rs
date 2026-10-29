use std::{
    fs::{File, Metadata},
    io,
    os::unix::fs::MetadataExt,
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
        let handle = File::open(path)?;
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
