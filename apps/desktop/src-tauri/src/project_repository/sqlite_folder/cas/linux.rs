use super::{changed, ordinary};
use crate::core::Result;
use std::{
    fs::{File, OpenOptions},
    io::Write,
    os::unix::fs::{FileExt, MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
};

pub(super) struct NewFile {
    file: File,
    path: PathBuf,
    device: u64,
    inode: u64,
    expected: Option<(String, usize)>,
}
impl NewFile {
    pub(super) fn create(directory: &Path, hash: &str) -> Result<Self> {
        let path = directory.join(hash);
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .custom_flags(0x2_0000)
            .open(&path)?;
        let metadata = file.metadata()?;
        if !metadata.is_file() || metadata.nlink() != 1 {
            return Err(changed());
        }
        let result = Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            file,
            path,
            expected: None,
        };
        result.recheck()?;
        Ok(result)
    }
    pub(super) fn write(&mut self, bytes: &[u8]) -> Result<()> {
        self.file.write_all(bytes)?;
        self.file.sync_all()?;
        self.expected = Some((crate::artifacts::digest(bytes), bytes.len()));
        Ok(())
    }
    pub(super) fn recheck(&self) -> Result<()> {
        let actual = self.file.metadata()?;
        let current = ordinary(&self.path)?;
        if !actual.is_file()
            || actual.nlink() != 1
            || current.nlink() != 1
            || actual.dev() != self.device
            || actual.ino() != self.inode
            || current.dev() != self.device
            || current.ino() != self.inode
            || actual.len() != current.len()
        {
            return Err(changed());
        }
        if let Some((hash, size)) = &self.expected {
            if actual.len() != *size as u64 {
                return Err(changed());
            }
            let mut bytes = vec![0; *size];
            self.file.read_exact_at(&mut bytes, 0)?;
            if crate::artifacts::digest(&bytes) != *hash {
                return Err(changed());
            }
        }
        Ok(())
    }
}

pub(super) fn sync_directory(directory: &Path) -> Result<()> {
    // The retained directory alias itself is a procfs symlink. The final '.'
    // must be a directory, never an accidentally opened database descriptor.
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(0x1_0000 | 0x2_0000)
        .open(directory.join("."))?;
    file.sync_all()?;
    Ok(())
}
