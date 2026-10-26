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

#[cfg(test)]
mod delete_rename_tests {
    use super::ProjectLock;

    /// Project deletion renames the project directory. The storage lock file lives inside
    /// it, so the lock must be released first: Windows denies renaming a directory that
    /// holds an open file (os error 5).
    #[test]
    fn project_directory_renames_after_the_lock_is_released() {
        let root = tempfile::tempdir().unwrap();
        let project = root.path().join("p1");
        std::fs::create_dir_all(project.join("metadata")).unwrap();
        let lock = ProjectLock::acquire(&project).unwrap();
        #[cfg(windows)]
        assert!(std::fs::rename(&project, root.path().join(".deleting-p1")).is_err());
        drop(lock);
        std::fs::rename(&project, root.path().join(".deleting-p1")).unwrap();
        std::fs::remove_dir_all(root.path().join(".deleting-p1")).unwrap();
    }
}
