//! Explicit Rust host decision, never deserialized from a project or IPC flag.
use super::super::{changed, FolderSession, FolderSnapshot, Opening, ReadBinding};
use crate::core::{AppError, Result};
use std::sync::Arc;

pub(in crate::project_repository::sqlite_folder) struct FolderAuthorization {
    opening: Arc<Opening>,
    binding: ReadBinding,
    saved: super::format::SavedStateStamp,
}

pub(super) fn required() -> AppError {
    AppError::Invalid("PERMISSION_REQUIRED".into())
}

impl FolderAuthorization {
    /// Trusted host entry after an explicit user/host decision to modify this
    /// source. Physical writer access is still independently acquired at commit.
    pub(in crate::project_repository::sqlite_folder) async fn authorize_modeling(
        session: &FolderSession,
        original: &FolderSnapshot,
    ) -> Result<Self> {
        let binding = session.read_binding(original).await?;
        binding.check_active()?;
        Ok(Self {
            opening: Arc::clone(&session.opening),
            binding,
            saved: original.saved.stamp(),
        })
    }
    pub(in crate::project_repository::sqlite_folder) fn check_capture(
        &self,
        session: &FolderSession,
        snapshot: &FolderSnapshot,
    ) -> Result<()> {
        self.check(session)?;
        session.owns(snapshot)?;
        if self.saved != snapshot.saved.stamp() {
            return Err(changed());
        }
        Ok(())
    }
    pub(super) fn check(&self, session: &FolderSession) -> Result<()> {
        session.check()?;
        if !Arc::ptr_eq(&self.opening, &session.opening) {
            return Err(changed());
        }
        self.binding.check_active()
    }
}
