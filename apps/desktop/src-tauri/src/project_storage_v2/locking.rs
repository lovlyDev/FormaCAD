use crate::core::{AppError, Result};
use std::{
    fs::{File, OpenOptions},
    io::{Seek, SeekFrom, Write},
    path::Path,
};

/// OS-held advisory lock: process termination releases it; never delete another owner's lock file.
pub struct ProjectLock {
    _file: File,
}
impl ProjectLock {
    pub fn acquire(root: &Path) -> Result<Self> {
        let path = crate::security::guarded(root, Path::new("metadata/storage-v2.lock"))?;
        std::fs::create_dir_all(path.parent().unwrap())?;
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?;
        file.try_lock()
            .map_err(|_| AppError::Invalid("Project storage is locked by another writer".into()))?;
        file.set_len(0)?;
        file.seek(SeekFrom::Start(0))?;
        write!(
            file,
            "pid={}\nacquiredAt={}\n",
            std::process::id(),
            chrono::Utc::now().to_rfc3339()
        )?;
        file.sync_all()?;
        Ok(Self { _file: file })
    }
}
