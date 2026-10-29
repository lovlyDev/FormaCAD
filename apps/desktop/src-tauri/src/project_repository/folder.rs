//! Read-only folder-v2 repository adapter. No app database, writer lease or recovery.
//! Lifecycle/location and saved-byte checks are optimistic observations, not a
//! lock against filesystem writers or a claim of NAS/USB durability.
use super::location::LocationIdentity;
use crate::{
    core::Result,
    models::{Project, ProjectFile},
    project_storage_v2::reader::FolderSnapshot,
};
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

struct Opening {
    id: uuid::Uuid,
    active: AtomicBool,
}

pub struct FolderRepository {
    root: PathBuf,
    project_id: String,
    location: LocationIdentity,
    opening: Arc<Opening>,
}

/// Private process-local authority tied to exactly one opening. Never serialized.
pub struct FolderReadSnapshot {
    opening: Arc<Opening>,
    saved: FolderSnapshot,
}

impl FolderRepository {
    pub fn open(root: &Path) -> Result<Self> {
        let location = LocationIdentity::capture(root, Path::new(""))?;
        let saved = FolderSnapshot::capture(root)?;
        if !location.recheck(root, Path::new(""))? {
            return Err(super::legacy::changed());
        }
        Ok(Self {
            root: root.into(),
            project_id: saved.project().id.clone(),
            location,
            opening: Arc::new(Opening {
                id: uuid::Uuid::new_v4(),
                active: AtomicBool::new(true),
            }),
        })
    }

    pub fn capture(&self) -> Result<FolderReadSnapshot> {
        self.check_open()?;
        let saved = FolderSnapshot::capture(&self.root)?;
        if saved.project().id != self.project_id {
            return Err(super::legacy::changed());
        }
        self.check_open()?;
        Ok(FolderReadSnapshot {
            opening: Arc::clone(&self.opening),
            saved,
        })
    }

    pub fn recheck(&self, snapshot: &FolderReadSnapshot) -> Result<()> {
        if !Arc::ptr_eq(&self.opening, &snapshot.opening) || self.opening.id != snapshot.opening.id
        {
            return Err(super::legacy::changed());
        }
        self.check_open()?;
        snapshot.saved.recheck(&self.root)?;
        self.check_open()
    }

    pub fn read_asset(&self, snapshot: &FolderReadSnapshot, file: &ProjectFile) -> Result<Vec<u8>> {
        self.recheck(snapshot)?;
        let bytes = snapshot.saved.read_asset(&self.root, file)?;
        self.recheck(snapshot)?;
        Ok(bytes)
    }

    pub fn read_history(&self, snapshot: &FolderReadSnapshot) -> Result<Option<Vec<u8>>> {
        self.recheck(snapshot)?;
        let bytes = snapshot.saved.read_history(&self.root)?;
        self.recheck(snapshot)?;
        Ok(bytes)
    }

    /// An old snapshot cannot become current again through close/reopen.
    pub fn close(&self) {
        self.opening.active.store(false, Ordering::Release);
    }

    fn check_open(&self) -> Result<()> {
        if !self.opening.active.load(Ordering::Acquire)
            || !self.location.recheck(&self.root, Path::new(""))?
            || !self.opening.active.load(Ordering::Acquire)
        {
            return Err(super::legacy::changed());
        }
        Ok(())
    }
}

impl Drop for FolderRepository {
    fn drop(&mut self) {
        self.close();
    }
}

impl FolderReadSnapshot {
    /// Captured metadata is immutable, but callers must recheck its authority
    /// through the original repository before accepting any delayed result.
    pub fn project(&self) -> &Project {
        self.saved.project()
    }
}

#[cfg(test)]
#[path = "folder/tests.rs"]
mod tests;
