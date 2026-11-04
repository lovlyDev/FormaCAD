//! A repository and its original capture travel together through async readers.
use super::{ProjectRepository, RepositorySnapshot};
use crate::{
    core::Result,
    models::{Project, ProjectFile},
};
use std::borrow::Cow;

pub struct RepositoryRead<'a> {
    repository: ProjectRepository<'a>,
    snapshot: RepositorySnapshot,
}

impl<'a> RepositoryRead<'a> {
    pub async fn capture(repository: ProjectRepository<'a>) -> Result<Self> {
        let snapshot = repository.capture().await?;
        Ok(Self {
            repository,
            snapshot,
        })
    }

    pub async fn capture_with_history(repository: ProjectRepository<'a>) -> Result<Self> {
        let snapshot = repository.capture_with_history().await?;
        Ok(Self {
            repository,
            snapshot,
        })
    }

    /// Managed/V2 retain project grouping; FCP1 uses its actual opening group.
    pub fn task_scope_id(&self) -> &str {
        match &self.snapshot.backend {
            super::CapturedBackend::Managed { snapshot, .. } => &snapshot.project.id,
            super::CapturedBackend::Folder(snapshot) => &snapshot.project().id,
            #[cfg(any(
                windows,
                all(
                    target_os = "linux",
                    any(target_arch = "x86_64", target_arch = "aarch64")
                )
            ))]
            super::CapturedBackend::Independent(snapshot) => snapshot.task_scope_id(),
        }
    }

    pub fn project(&self) -> &Project {
        self.snapshot.project()
    }

    pub async fn recheck(&self) -> Result<()> {
        self.repository.recheck(&self.snapshot).await
    }

    pub async fn read_asset(&self, file: &ProjectFile) -> Result<Vec<u8>> {
        self.repository.read_asset(&self.snapshot, file).await
    }

    pub async fn read_history(&self) -> Result<Option<Cow<'_, [u8]>>> {
        self.repository.read_history(&self.snapshot).await
    }

    /// Request-local revocation never closes a window-owned shared opening.
    pub fn close(&self) {
        self.repository.close();
    }
}

#[cfg(any(
    windows,
    all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )
))]
impl RepositoryRead<'static> {
    /// Adopt the original host capture without reopening or taking a newer view.
    /// Passing another opening's view fails before this reader is returned.
    pub async fn from_folder_view(
        source: std::sync::Arc<crate::project_repository::sqlite_folder::api::FolderProject>,
        original: crate::project_repository::sqlite_folder::api::ReadView,
    ) -> Result<Self> {
        let repository = ProjectRepository::from_folder_source(source);
        let snapshot = RepositorySnapshot {
            backend: super::CapturedBackend::Independent(Box::new(original)),
            repository: repository.identity,
        };
        repository.recheck(&snapshot).await?;
        Ok(Self {
            repository,
            snapshot,
        })
    }
}
