//! Original window-selected source and view kept together across host decisions.
//! These Rust values are not IPC grants and are never serialized or persisted.
use super::WindowSources;
use crate::{
    core::Result,
    models::Project,
    project_history::Direction,
    project_repository::{
        backend::RepositoryRead,
        sqlite_folder::api::{
            FolderProject, ModelChange, ModelIntent, ModelRuntime, Publication, ReadView,
            WriteApproval,
        },
    },
};
use std::{path::Path, sync::Arc};

/// The original Arc and view cannot be replaced independently after an await.
/// Capturing this value confers no writer lease or permission approval.
pub struct FolderOperation {
    source: Arc<FolderProject>,
    view: ReadView,
}

impl WindowSources {
    pub async fn capture_operation(
        &self,
        label: &str,
        key: &str,
        project_id: &str,
        expected_commit: &str,
    ) -> Result<FolderOperation> {
        let source = self.existing(label)?.lookup(key)?;
        let view = source.capture_metadata().await?;
        if view.project().id != project_id || view.commit_id() != expected_commit {
            return Err(super::changed());
        }
        Ok(FolderOperation { source, view })
    }
}

impl FolderOperation {
    pub fn project(&self) -> &Project {
        self.view.project()
    }
    pub fn commit_id(&self) -> &str {
        self.view.commit_id()
    }
    pub fn task_scope_id(&self) -> &str {
        self.view.task_scope_id()
    }
    pub async fn recheck(&self) -> Result<()> {
        self.source.recheck(&self.view).await
    }
    pub async fn into_read(self) -> Result<RepositoryRead<'static>> {
        RepositoryRead::from_folder_view(self.source, self.view).await
    }

    /// Trusted host only: call AFTER its explicit permission decision for this
    /// operation. A renderer source key or bool is not such a decision. The
    /// backend verifies full history here and acquires the OS lease at commit.
    pub async fn approve_write(&self) -> Result<WriteApproval> {
        self.source.approve_write(&self.view).await
    }

    /// Host-prepared typed candidate only; not a raw renderer project-save API.
    /// Approval and original saved state are rechecked by the existing publisher.
    pub async fn commit(
        self,
        approval: WriteApproval,
        candidate: Project,
        assets: Vec<(String, Vec<u8>)>,
    ) -> Result<Publication> {
        self.source
            .commit(&self.view, approval, candidate, assets)
            .await
    }

    /// None preserves the existing pure no-op path; a real change requires the
    /// opaque host approval. No path lookup or fresh view substitutes the original.
    pub async fn apply_model(
        self,
        approval: Option<WriteApproval>,
        intent: ModelIntent,
        runtime: ModelRuntime<'_>,
    ) -> Result<ModelChange> {
        self.source
            .apply_model(&self.view, approval, intent, runtime)
            .await
    }
    pub async fn apply_history(
        self,
        approval: WriteApproval,
        direction: Direction,
    ) -> Result<Publication> {
        self.source
            .apply_history(&self.view, approval, direction)
            .await
    }

    /// The host separately validates the selected destination. Copying does not
    /// approve writing the original, even when another writer owns its OS lease.
    pub async fn save_copy(self, destination: &Path) -> Result<Publication> {
        self.source.save_copy(&self.view, destination).await
    }
}

#[cfg(test)]
mod tests;
