use crate::{core::Result, security};
use std::path::{Path, PathBuf};
pub(super) struct Workspace {
    root: PathBuf,
    relative: PathBuf,
    pub path: PathBuf,
}
impl Workspace {
    pub fn create(root: &Path, project_id: &str) -> Result<Self> {
        security::valid_id(project_id)?;
        let parent = Path::new(".transient/reference-measurements");
        std::fs::create_dir_all(security::guarded(root, parent)?)?;
        let relative = parent.join(format!("{project_id}-{}", uuid::Uuid::new_v4()));
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
        match security::guarded(&self.root, &self.relative) {
            Ok(path) => {
                if let Err(error) = std::fs::remove_dir_all(path) {
                    tracing::warn!(%error, "Temporary reference measurement cleanup failed");
                }
            }
            Err(error) => {
                tracing::warn!(%error, "Temporary reference measurement cleanup guard failed")
            }
        }
    }
}
