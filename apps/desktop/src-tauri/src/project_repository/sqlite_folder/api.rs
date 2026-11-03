//! Rust host boundary for an explicitly selected independent FCP1 source.
//! No AppState or global app index is required. These values are not IPC data.
use super::{
    format,
    session::{FolderSession, FolderSnapshot},
};
use crate::{
    core::Result,
    models::{Project, ProjectFile},
};
use std::path::Path;

mod migration;
mod model;
pub use migration::{BackupRecord, MigrationReport, V2Backup};
mod publication;
mod write;
pub use model::{ModelChange, ModelIntent, ModelRuntime};
pub use publication::{CommitPresence, Publication, SavedCommit};
pub use write::WriteApproval;
#[cfg(feature = "native-worker-client")]
mod step;
#[cfg(feature = "native-worker-client")]
pub use step::{SelectedStepFile, StepDestinationApproval, StepProjectIntent, StepRead};

/// One physical opening. Closing it revokes every previously captured view.
pub struct FolderProject {
    session: FolderSession,
}

/// An opaque view tied to its actual opening and saved state, never an access grant.
pub struct ReadView {
    snapshot: FolderSnapshot,
}
impl ReadView {
    pub fn project(&self) -> &Project {
        self.snapshot.project()
    }
    pub fn commit_id(&self) -> &str {
        self.snapshot.commit_id()
    }
    pub fn history_state(&self) -> &crate::project_history::HistoryState {
        self.snapshot.history_state()
    }
}

impl FolderProject {
    pub async fn open(root: &Path) -> Result<Self> {
        Ok(Self {
            session: FolderSession::open_metadata(root).await?,
        })
    }
    pub async fn capture(&self) -> Result<ReadView> {
        Ok(ReadView {
            snapshot: self.session.capture().await?,
        })
    }
    /// Capture project/cursor and declared history seal without reading events.
    /// History is verified by read_history, write approval, or complete capture.
    pub async fn capture_metadata(&self) -> Result<ReadView> {
        Ok(ReadView {
            snapshot: self.session.capture_metadata().await?,
        })
    }
    pub async fn recheck(&self, original: &ReadView) -> Result<()> {
        self.session.recheck(&original.snapshot).await
    }
    pub async fn read_asset(&self, original: &ReadView, file: &ProjectFile) -> Result<Vec<u8>> {
        self.session.read_asset(&original.snapshot, file).await
    }
    pub async fn read_history(&self, original: &ReadView) -> Result<Vec<u8>> {
        self.session.read_history(&original.snapshot).await
    }
    pub async fn resolve_commit(
        &self,
        original: &ReadView,
        commit_id: &str,
    ) -> Result<CommitPresence> {
        Ok(
            match self
                .session
                .resolve_commit(&original.snapshot, commit_id)
                .await?
            {
                format::CommitPresence::Present { is_head } => CommitPresence::Present { is_head },
                format::CommitPresence::Absent => CommitPresence::Absent,
            },
        )
    }
    pub fn close(&self) {
        self.session.close();
    }
}

#[cfg(test)]
mod metadata_tests;
#[cfg(test)]
mod tests;
#[cfg(all(test, feature = "native-worker-client"))]
mod worker_tests;
