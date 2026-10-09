use crate::{core::Result, security};
use std::path::{Path, PathBuf};
/// Generated geometry outside project directories, including read-only sources.
pub struct Workspace {
    root: PathBuf,
    relative: PathBuf,
    pub path: PathBuf,
}
impl Workspace {
    pub fn create(root: &Path) -> Result<Self> {
        std::fs::create_dir_all(security::guarded(root, Path::new(".transient"))?)?;
        let relative = PathBuf::from(format!(".transient/export-{}", uuid::Uuid::new_v4()));
        let path = security::guarded(root, &relative)?;
        std::fs::create_dir(&path)?;
        Ok(Self {
            root: root.into(),
            relative,
            path,
        })
    }
}
impl Drop for Workspace {
    fn drop(&mut self) {
        if let Ok(path) = security::guarded(&self.root, &self.relative) {
            if let Err(error) = std::fs::remove_dir_all(path) {
                tracing::warn!(%error,"Temporary export cleanup failed");
            }
        }
    }
}
