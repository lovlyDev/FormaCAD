//! Original lifecycle/location checks, deliberately separate from saved-state seals.
use super::{Backend, CapturedBackend, ProjectRepository, RepositorySnapshot};
use crate::{core::Result, project_repository::legacy};

impl ProjectRepository<'_> {
    pub(crate) async fn recheck_source(&self, snapshot: &RepositorySnapshot) -> Result<()> {
        self.check_open()?;
        if snapshot.repository != self.identity {
            return Err(legacy::changed());
        }
        match (&self.backend, &snapshot.backend) {
            (Backend::Managed { state, project_id }, CapturedBackend::Managed { snapshot, .. })
                if &snapshot.project.id == project_id =>
            {
                legacy::recheck_lifecycle(state, snapshot)
            }
            (Backend::Folder(source), CapturedBackend::Folder(snapshot)) => {
                source.recheck_source(snapshot)
            }
            #[cfg(any(
                windows,
                all(
                    target_os = "linux",
                    any(target_arch = "x86_64", target_arch = "aarch64")
                )
            ))]
            (Backend::Independent { source, .. }, CapturedBackend::Independent(snapshot)) => {
                source.recheck_source(snapshot).await
            }
            _ => Err(legacy::changed()),
        }?;
        self.check_open()
    }
}
