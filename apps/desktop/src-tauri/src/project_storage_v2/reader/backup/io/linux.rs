//! Linux output components are relative to retained directory descriptors.
use super::{changed, Result};
use std::{
    fs::{File, OpenOptions},
    io::Write,
    os::{
        fd::AsRawFd,
        unix::fs::{MetadataExt, OpenOptionsExt},
    },
    path::{Path, PathBuf},
};

const O_DIRECTORY: i32 = 0o200000;
const O_NOFOLLOW: i32 = 0o400000;

pub(super) struct Directory {
    file: File,
    entry: PathBuf,
    device: u64,
    inode: u64,
}
impl Directory {
    pub(super) fn open(path: &Path) -> Result<Self> {
        // A lease root may be an intentional procfs descriptor alias. Its final
        // '.' opens the pinned directory rather than following an untrusted leaf.
        let entry = path.join(".");
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(O_DIRECTORY | O_NOFOLLOW)
            .open(&entry)?;
        let metadata = file.metadata()?;
        if !metadata.is_dir() {
            return Err(changed());
        }
        Ok(Self {
            file,
            entry,
            device: metadata.dev(),
            inode: metadata.ino(),
        })
    }
    fn alias(&self) -> PathBuf {
        PathBuf::from(format!("/proc/self/fd/{}", self.file.as_raw_fd()))
    }
    pub(super) fn create_directory(&self, name: &str) -> Result<Self> {
        self.recheck()?;
        let path = self.alias().join(name);
        std::fs::create_dir(&path)?;
        // Open the actual named component with NOFOLLOW before appending '.'.
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(O_DIRECTORY | O_NOFOLLOW)
            .open(&path)?;
        let metadata = file.metadata()?;
        if !metadata.is_dir() {
            return Err(changed());
        }
        let child = Self {
            file,
            entry: path,
            device: metadata.dev(),
            inode: metadata.ino(),
        };
        child.recheck()?;
        self.recheck()?;
        Ok(child)
    }
    pub(super) fn recheck(&self) -> Result<()> {
        let current = OpenOptions::new()
            .read(true)
            .custom_flags(O_DIRECTORY | O_NOFOLLOW)
            .open(&self.entry)?;
        for metadata in [self.file.metadata()?, current.metadata()?] {
            if !metadata.is_dir() || metadata.dev() != self.device || metadata.ino() != self.inode {
                return Err(changed());
            }
        }
        Ok(())
    }
    pub(super) fn write_new(&self, name: &str, bytes: &[u8]) -> Result<()> {
        self.recheck()?;
        let path = self.alias().join(name);
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .custom_flags(O_NOFOLLOW)
            .open(&path)?;
        let before = file.metadata()?;
        if !before.is_file() || before.nlink() != 1 {
            return Err(changed());
        }
        file.write_all(bytes)?;
        file.sync_all()?;
        let after = file.metadata()?;
        let named = std::fs::symlink_metadata(path)?;
        for metadata in [after, named] {
            if !metadata.is_file()
                || metadata.nlink() != 1
                || metadata.dev() != before.dev()
                || metadata.ino() != before.ino()
                || metadata.len() != bytes.len() as u64
            {
                return Err(changed());
            }
        }
        self.recheck()
    }
    pub(super) fn sync(&self) -> Result<()> {
        self.recheck()?;
        self.file.sync_all()?;
        self.recheck()
    }
}
