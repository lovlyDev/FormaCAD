//! One live physical FCP1 opening. Saved snapshots cannot authorize another
//! opening even when repository/project/commit UUIDs were copied verbatim.
use super::{format, read_only::ReadOnlyDatabase};
use crate::{
    core::{AppError, Result},
    models::{Project, ProjectFile},
};
use std::{
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tokio::sync::Mutex;

struct Opening {
    active: AtomicBool,
}

pub(super) struct FolderSession {
    opening: Arc<Opening>,
    database: Mutex<ReadOnlyDatabase>,
    repository_id: String,
    project_id: String,
}

pub(super) struct FolderSnapshot {
    opening: Arc<Opening>,
    saved: format::Snapshot,
}

impl FolderSnapshot {
    pub(super) fn project(&self) -> &Project {
        self.saved.project()
    }
}

fn changed() -> AppError {
    AppError::Invalid("PROJECT_SNAPSHOT_CHANGED".into())
}

impl FolderSession {
    pub(super) async fn open(root: &Path) -> Result<Self> {
        let mut database = ReadOnlyDatabase::open(root).await?;
        let saved = format::read(database.conn()).await?;
        database.recheck().await?;
        Ok(Self {
            opening: Arc::new(Opening {
                active: AtomicBool::new(true),
            }),
            database: Mutex::new(database),
            repository_id: saved.expectation().repository_id().into(),
            project_id: saved.expectation().project_id().into(),
        })
    }

    fn check(&self) -> Result<()> {
        if self.opening.active.load(Ordering::Acquire) {
            Ok(())
        } else {
            Err(changed())
        }
    }

    fn owns(&self, snapshot: &FolderSnapshot) -> Result<()> {
        self.check()?;
        if Arc::ptr_eq(&self.opening, &snapshot.opening) {
            Ok(())
        } else {
            Err(changed())
        }
    }

    async fn capture_locked(&self, database: &mut ReadOnlyDatabase) -> Result<format::Snapshot> {
        self.check()?;
        database.recheck().await?;
        self.check()?;
        let saved = format::read(database.conn()).await?;
        self.check()?;
        database.recheck().await?;
        self.check()?;
        if saved.expectation().repository_id() != self.repository_id
            || saved.expectation().project_id() != self.project_id
        {
            return Err(changed());
        }
        Ok(saved)
    }

    pub(super) async fn capture(&self) -> Result<FolderSnapshot> {
        self.check()?;
        let mut database = self.database.lock().await;
        let saved = self.capture_locked(&mut database).await?;
        Ok(FolderSnapshot {
            opening: Arc::clone(&self.opening),
            saved,
        })
    }

    async fn recheck_locked(
        &self,
        database: &mut ReadOnlyDatabase,
        snapshot: &FolderSnapshot,
    ) -> Result<()> {
        self.owns(snapshot)?;
        let current = self.capture_locked(database).await?;
        if !snapshot.saved.same_saved_state(&current) {
            return Err(changed());
        }
        self.owns(snapshot)
    }

    pub(super) async fn recheck(&self, snapshot: &FolderSnapshot) -> Result<()> {
        self.owns(snapshot)?;
        let mut database = self.database.lock().await;
        self.recheck_locked(&mut database, snapshot).await
    }

    pub(super) async fn read_history(&self, snapshot: &FolderSnapshot) -> Result<Vec<u8>> {
        self.recheck(snapshot).await?;
        Ok(snapshot.saved.history().to_vec())
    }

    /// Resolve a known attempted commit through the original opening. The old
    /// saved state may have advanced; its physical/opening authority must not.
    pub(super) async fn resolve_commit(
        &self,
        snapshot: &FolderSnapshot,
        commit_id: &str,
    ) -> Result<format::CommitPresence> {
        self.owns(snapshot)?;
        let mut database = self.database.lock().await;
        self.owns(snapshot)?;
        database.recheck().await?;
        self.owns(snapshot)?;
        let presence =
            format::resolve_commit(database.conn(), snapshot.saved.expectation(), commit_id)
                .await?;
        self.owns(snapshot)?;
        database.recheck().await?;
        self.owns(snapshot)?;
        Ok(presence)
    }

    pub(super) async fn read_asset(
        &self,
        snapshot: &FolderSnapshot,
        file: &ProjectFile,
    ) -> Result<Vec<u8>> {
        self.owns(snapshot)?;
        // Resolve through the captured descriptor; never accept an arbitrary
        // caller hash/size merely because it is a valid CAS filename.
        let descriptor = snapshot
            .project()
            .files
            .iter()
            .find(|saved| *saved == file)
            .ok_or_else(changed)?;
        let hash = descriptor.sha256.as_deref().ok_or_else(changed)?;
        let mut database = self.database.lock().await;
        self.recheck_locked(&mut database, snapshot).await?;
        let bytes = database.read_blob(hash, descriptor.size)?;
        self.recheck_locked(&mut database, snapshot).await?;
        Ok(bytes)
    }

    /// Invalidate immediately, including operations awaiting the connection gate.
    /// The SQLx connection may finish physical close later; no result is valid.
    pub(super) fn close(&self) {
        self.opening.active.store(false, Ordering::Release);
    }
}

impl Drop for FolderSession {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
mod tests;
