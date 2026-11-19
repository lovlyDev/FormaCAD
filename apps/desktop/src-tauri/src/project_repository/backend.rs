//! Explicit read dispatch over managed SQL and independent folder sources.
//! Snapshots retain their original backend and opening; none is a writer lease.
use super::{
    folder::{FolderReadSnapshot, FolderRepository},
    legacy::{self, CommittedSnapshot},
};
use crate::{
    core::{AppError, AppState, Result},
    models::{Project, ProjectFile},
};
use std::{borrow::Cow, path::Path};

enum Backend<'a> {
    Managed {
        state: &'a AppState,
        project_id: String,
    },
    Folder(FolderRepository),
}

enum CapturedBackend {
    Managed {
        snapshot: Box<CommittedSnapshot>,
        with_archive: bool,
    },
    Folder(Box<FolderReadSnapshot>),
}

/// A read handle to one explicitly selected source. Constructing a managed
/// handle does not open the project's writer access or close existing tabs.
pub struct ProjectRepository<'a> {
    backend: Backend<'a>,
    identity: uuid::Uuid,
}

/// Immutable public data view with private backend/session authority. Tokens
/// are never serialized, accepted from UI, or transferable to another handle.
pub struct RepositorySnapshot {
    backend: CapturedBackend,
    repository: uuid::Uuid,
}

impl RepositorySnapshot {
    pub fn project(&self) -> &Project {
        match &self.backend {
            CapturedBackend::Managed { snapshot, .. } => &snapshot.project,
            CapturedBackend::Folder(snapshot) => snapshot.project(),
        }
    }
}

impl<'a> ProjectRepository<'a> {
    pub fn managed(state: &'a AppState, project_id: &str) -> Result<Self> {
        crate::security::valid_id(project_id)?;
        crate::security::guarded(&state.root, Path::new(project_id))?;
        Ok(Self {
            backend: Backend::Managed {
                state,
                project_id: project_id.into(),
            },
            identity: uuid::Uuid::new_v4(),
        })
    }

    pub fn folder(root: &Path) -> Result<Self> {
        Ok(Self {
            backend: Backend::Folder(FolderRepository::open(root)?),
            identity: uuid::Uuid::new_v4(),
        })
    }

    /// Capture project metadata without loading the SQL event archive.
    pub async fn capture(&self) -> Result<RepositorySnapshot> {
        self.capture_inner(false).await
    }

    /// Capture the managed archive in the same saved SQL snapshot. Folder
    /// metadata already binds its archive checksum, so archive bytes stay lazy.
    pub async fn capture_with_history(&self) -> Result<RepositorySnapshot> {
        self.capture_inner(true).await
    }

    async fn capture_inner(&self, with_archive: bool) -> Result<RepositorySnapshot> {
        let backend = match &self.backend {
            Backend::Managed { state, project_id } => CapturedBackend::Managed {
                snapshot: Box::new(if with_archive {
                    legacy::capture_with_history(state, project_id).await?
                } else {
                    legacy::capture_project(state, project_id).await?
                }),
                with_archive,
            },
            Backend::Folder(repository) => CapturedBackend::Folder(Box::new(repository.capture()?)),
        };
        Ok(RepositorySnapshot {
            backend,
            repository: self.identity,
        })
    }

    /// Revalidate the ORIGINAL captured authority and saved state. This is an
    /// optimistic check; it does not block later external modifications.
    pub async fn recheck(&self, snapshot: &RepositorySnapshot) -> Result<()> {
        if snapshot.repository != self.identity {
            return Err(legacy::changed());
        }
        match (&self.backend, &snapshot.backend) {
            (Backend::Managed { state, project_id }, CapturedBackend::Managed { snapshot, .. })
                if &snapshot.project.id == project_id =>
            {
                legacy::recheck(state, snapshot).await
            }
            (Backend::Folder(repository), CapturedBackend::Folder(snapshot)) => {
                repository.recheck(snapshot)
            }
            _ => Err(legacy::changed()),
        }
    }

    /// Read only a descriptor that exactly belongs to this captured project.
    /// Declared sizes are bounded before allocation; embedded legacy data also
    /// retains exact size and optional checksum checks after decoding.
    pub async fn read_asset(
        &self,
        snapshot: &RepositorySnapshot,
        file: &ProjectFile,
    ) -> Result<Vec<u8>> {
        self.recheck(snapshot).await?;
        if !snapshot.project().files.contains(file)
            || file.size > crate::artifacts::MAX_FILE_BYTES as u64
        {
            return Err(invalid_asset());
        }
        let bytes = match (&self.backend, &snapshot.backend) {
            (Backend::Managed { state, project_id }, CapturedBackend::Managed { .. }) => {
                crate::artifacts::read(state, project_id, file)?
            }
            (Backend::Folder(repository), CapturedBackend::Folder(snapshot)) => {
                repository.read_asset(snapshot, file)?
            }
            _ => return Err(legacy::changed()),
        };
        if bytes.len() as u64 != file.size
            || file.sha256.as_deref().is_some_and(|hash| {
                !crate::artifacts::valid_digest(hash) || crate::artifacts::digest(&bytes) != hash
            })
        {
            return Err(invalid_asset());
        }
        self.recheck(snapshot).await?;
        Ok(bytes)
    }

    /// SQL callers must explicitly capture_with_history: a new archive capture
    /// cannot retroactively authorize a metadata-only snapshot. Folder captures
    /// already bind the immutable history entry and can read it on demand.
    pub async fn read_history<'s>(
        &self,
        snapshot: &'s RepositorySnapshot,
    ) -> Result<Option<Cow<'s, [u8]>>> {
        self.recheck(snapshot).await?;
        let history = match (&self.backend, &snapshot.backend) {
            (
                Backend::Managed { .. },
                CapturedBackend::Managed {
                    snapshot,
                    with_archive: true,
                },
            ) => snapshot.history.as_deref().map(Cow::Borrowed),
            (
                Backend::Managed { .. },
                CapturedBackend::Managed {
                    with_archive: false,
                    ..
                },
            ) => {
                return Err(AppError::Invalid("HISTORY_INVALID_STATE".into()));
            }
            (Backend::Folder(repository), CapturedBackend::Folder(snapshot)) => {
                repository.read_history(snapshot)?.map(Cow::Owned)
            }
            _ => return Err(legacy::changed()),
        };
        self.recheck(snapshot).await?;
        Ok(history)
    }
}

fn invalid_asset() -> AppError {
    AppError::Invalid("Project file failed its integrity check".into())
}

#[cfg(test)]
#[path = "backend/tests.rs"]
mod tests;
