//! One live physical FCP1 opening. Saved snapshots cannot authorize another
//! opening even when repository/project/commit UUIDs were copied verbatim.
use super::{format, read_only::ReadOnlyDatabase};
use crate::{
    core::{AppError, Result},
    models::Project,
};
use std::{
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tokio::sync::Mutex;

pub(super) mod apply;
mod binding;
mod execution_assets;
mod history;
mod read;
mod write;
pub(super) use binding::ReadBinding;

struct Opening {
    active: AtomicBool,
    task_id: String,
}

pub(super) struct FolderSession {
    root: std::path::PathBuf,
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
    fn scope(&self) -> format::ReadScope {
        if self.saved.history().is_some() {
            format::ReadScope::Complete
        } else {
            format::ReadScope::Metadata
        }
    }
    pub(super) fn project(&self) -> &Project {
        self.saved.project()
    }

    pub(super) fn history_state(&self) -> &crate::project_history::HistoryState {
        self.saved.history_state()
    }

    pub(super) fn commit_id(&self) -> &str {
        self.saved.expectation().commit_id()
    }

    pub(super) fn same_saved_state(&self, expected: &format::Snapshot) -> bool {
        self.saved.same_saved_state(expected)
    }
}

fn changed() -> AppError {
    AppError::Invalid("PROJECT_SNAPSHOT_CHANGED".into())
}

impl FolderSession {
    pub(super) async fn open(root: &Path) -> Result<Self> {
        Self::open_scoped(root, format::ReadScope::Complete).await
    }

    pub(super) async fn open_metadata(root: &Path) -> Result<Self> {
        Self::open_scoped(root, format::ReadScope::Metadata).await
    }

    async fn open_scoped(root: &Path, scope: format::ReadScope) -> Result<Self> {
        let mut database = ReadOnlyDatabase::open(root).await?;
        let saved = match scope {
            format::ReadScope::Complete => format::read(database.conn()).await?,
            format::ReadScope::Metadata => format::read_metadata(database.conn()).await?,
        };
        database.recheck().await?;
        Ok(Self {
            root: std::path::absolute(root)?,
            opening: Arc::new(Opening {
                active: AtomicBool::new(true),
                task_id: uuid::Uuid::new_v4().to_string(),
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
