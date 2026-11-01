//! Folder history uses the same pure planner as managed storage. Publication
//! still requires the original exact snapshot and the real session write route.
use super::super::format;
use super::{FolderSession, FolderSnapshot};
use crate::{core::Result, project_history::Direction};

impl FolderSession {
    /// Trusted host API; permission and source task registration belong to the
    /// host. A UI revision ID alone cannot construct the captured authority.
    pub(in crate::project_repository::sqlite_folder) async fn apply_history(
        &self,
        original: &FolderSnapshot,
        direction: Direction,
    ) -> Result<format::CommitOutcome> {
        self.owns(original)?;
        let (candidate, intent) = crate::project_history::prepare(
            original.project(),
            original.saved.history_state(),
            direction,
            original.project().current_revision.as_deref(),
            &uuid::Uuid::new_v4().to_string(),
            &chrono::Utc::now().to_rfc3339(),
        )?;
        self.commit(original, candidate, vec![], Some(intent)).await
    }
}

#[cfg(test)]
mod tests;
