//! Unserializable physical authority minted only by the owning FolderSession.
use super::super::read_only::source::Source;
use super::{changed, Opening};
use crate::core::Result;
use std::sync::{atomic::Ordering, Arc};
pub(crate) struct ReadBinding {
    source: Arc<Source>,
    opening: Arc<Opening>,
}
impl ReadBinding {
    pub(super) fn new(source: Arc<Source>, opening: Arc<Opening>) -> Self {
        Self { source, opening }
    }
    pub(in crate::project_repository::sqlite_folder) fn check_active(&self) -> Result<()> {
        if !self.opening.active.load(Ordering::Acquire) {
            return Err(changed());
        }
        self.source.check_binding()
    }
    pub(in crate::project_repository::sqlite_folder) fn check_root(
        &self,
        root: &std::path::Path,
    ) -> Result<()> {
        self.check_active()?;
        self.source.check_writer_location(root)
    }
    pub(in crate::project_repository::sqlite_folder) fn check_location(&self) -> Result<()> {
        self.source.check_binding()
    }
    /// SAFETY: live underlying SQLite C-ABI file, exclusive during initial xOpen.
    pub(in crate::project_repository::sqlite_folder) unsafe fn attest_base(
        &self,
        base: *mut std::ffi::c_void,
    ) -> Result<()> {
        self.check_active()?;
        unsafe { self.source.attest_writer_base(base) }?;
        self.check_active()
    }
}
