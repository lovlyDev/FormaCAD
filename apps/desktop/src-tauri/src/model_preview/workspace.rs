use crate::{core::Result, security};
use std::path::{Path, PathBuf};
/// A preview owns an app-level transient directory, including for read-only projects.
pub(super) struct Workspace {
    root: PathBuf,
    relative: PathBuf,
    pub path: PathBuf,
}
impl Workspace {
    pub fn create(root: &Path, project_id: &str) -> Result<Self> {
        security::valid_id(project_id)?;
        let parent = PathBuf::from(".transient/previews");
        let guarded = security::guarded(root, &parent)?;
        std::fs::create_dir_all(guarded)?;
        let relative = parent.join(format!("{project_id}-{}", uuid::Uuid::new_v4()));
        let path = security::guarded(root, &relative)?;
        std::fs::create_dir(&path)?;
        Ok(Self {
            root: root.to_path_buf(),
            relative,
            path,
        })
    }
}
impl Drop for Workspace {
    fn drop(&mut self) {
        if let Ok(path) = security::guarded(&self.root, &self.relative) {
            if let Err(error) = std::fs::remove_dir_all(path) {
                tracing::warn!(%error,"Could not remove temporary model preview");
            }
        }
    }
}
