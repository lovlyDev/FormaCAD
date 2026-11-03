//! Explicit trusted-host transaction entry points. No grants are deserialized.
use super::super::{copy, creation, session::apply::FolderAuthorization};
use super::{FolderProject, Publication, ReadView};
use crate::{core::Result, models::Project, project_history::Direction};
use std::path::Path;

/// A single host decision for the exact original saved state. The real OS lease
/// is independently acquired at publication; this object does not claim it.
pub struct WriteApproval {
    pub(super) authorization: FolderAuthorization,
}
impl WriteApproval {
    pub(super) fn check(&self, source: &FolderProject, original: &ReadView) -> Result<()> {
        self.authorization
            .check_capture(&source.session, &original.snapshot)
    }
}

impl FolderProject {
    /// Invoke only after the host's explicit modify decision for this read view.
    pub async fn approve_write(&self, original: &ReadView) -> Result<WriteApproval> {
        let authorization =
            FolderAuthorization::authorize_modeling(&self.session, &original.snapshot).await?;
        Ok(WriteApproval { authorization })
    }
    /// The selected target must already be empty. Failure never permits replay.
    pub async fn create(
        root: &Path,
        project: Project,
        assets: Vec<(String, Vec<u8>)>,
        history: Option<Vec<u8>>,
    ) -> Result<Publication> {
        Ok(creation::create(root, project, assets, history)
            .await?
            .into())
    }
    /// Transactional backend operation for a trusted prepared command candidate.
    pub async fn commit(
        &self,
        original: &ReadView,
        approval: WriteApproval,
        candidate: Project,
        assets: Vec<(String, Vec<u8>)>,
    ) -> Result<Publication> {
        approval.check(self, original)?;
        Ok(self
            .session
            .commit(&original.snapshot, candidate, assets, None)
            .await?
            .into())
    }
    pub async fn apply_history(
        &self,
        original: &ReadView,
        approval: WriteApproval,
        direction: Direction,
    ) -> Result<Publication> {
        approval.check(self, original)?;
        Ok(self
            .session
            .apply_history(&original.snapshot, direction)
            .await?
            .into())
    }
    /// No writer approval is requested from the source for an independent copy.
    pub async fn save_copy(&self, original: &ReadView, destination: &Path) -> Result<Publication> {
        Ok(
            copy::save_copy(&self.session, &original.snapshot, destination)
                .await?
                .into(),
        )
    }
}
