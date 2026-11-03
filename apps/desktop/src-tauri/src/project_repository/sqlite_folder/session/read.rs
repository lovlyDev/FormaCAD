//! Scoped source reads. Metadata keeps declared seals; full reads validate events.
use super::{changed, format, FolderSession, FolderSnapshot, ReadBinding, ReadOnlyDatabase};
use crate::{core::Result, models::ProjectFile};
use std::sync::Arc;

impl FolderSession {
    async fn capture_locked(&self, database: &mut ReadOnlyDatabase) -> Result<format::Snapshot> {
        self.capture_locked_scoped(database, format::ReadScope::Complete)
            .await
    }

    async fn capture_locked_scoped(
        &self,
        database: &mut ReadOnlyDatabase,
        scope: format::ReadScope,
    ) -> Result<format::Snapshot> {
        self.check()?;
        database.recheck().await?;
        self.check()?;
        let saved = match scope {
            format::ReadScope::Complete => format::read(database.conn()).await?,
            format::ReadScope::Metadata => format::read_metadata(database.conn()).await?,
        };
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

    pub(in crate::project_repository::sqlite_folder) async fn capture(
        &self,
    ) -> Result<FolderSnapshot> {
        self.capture_scoped(format::ReadScope::Complete).await
    }

    pub(in crate::project_repository::sqlite_folder) async fn capture_metadata(
        &self,
    ) -> Result<FolderSnapshot> {
        self.capture_scoped(format::ReadScope::Metadata).await
    }

    async fn capture_scoped(&self, scope: format::ReadScope) -> Result<FolderSnapshot> {
        self.check()?;
        let mut database = self.database.lock().await;
        let saved = self.capture_locked_scoped(&mut database, scope).await?;
        Ok(FolderSnapshot {
            opening: Arc::clone(&self.opening),
            saved,
        })
    }

    /// Preserve concurrent metadata only after proving a continuation of the
    /// same model through this original physical opening. This does not replace
    /// the job's original snapshot or authorize a different source.
    pub(in crate::project_repository::sqlite_folder) async fn latest_compatible(
        &self,
        original: &FolderSnapshot,
    ) -> Result<FolderSnapshot> {
        self.owns(original)?;
        let mut database = self.database.lock().await;
        database.recheck().await?;
        self.owns(original)?;
        let saved = format::read_compatible(database.conn(), &original.saved).await?;
        self.owns(original)?;
        database.recheck().await?;
        self.owns(original)?;
        Ok(FolderSnapshot {
            opening: Arc::clone(&self.opening),
            saved,
        })
    }

    pub(super) async fn recheck_locked(
        &self,
        database: &mut ReadOnlyDatabase,
        snapshot: &FolderSnapshot,
    ) -> Result<()> {
        self.recheck_locked_scoped(database, snapshot, format::ReadScope::Complete)
            .await
    }

    async fn recheck_locked_scoped(
        &self,
        database: &mut ReadOnlyDatabase,
        snapshot: &FolderSnapshot,
        scope: format::ReadScope,
    ) -> Result<()> {
        self.owns(snapshot)?;
        let current = self.capture_locked_scoped(database, scope).await?;
        if !snapshot.saved.same_saved_state(&current) {
            return Err(changed());
        }
        self.owns(snapshot)
    }

    /// Mint authority only under this opening's original reader gate.
    pub(in crate::project_repository::sqlite_folder) async fn read_binding(
        &self,
        snapshot: &FolderSnapshot,
    ) -> Result<ReadBinding> {
        self.owns(snapshot)?;
        let mut database = self.database.lock().await;
        self.recheck_locked(&mut database, snapshot).await?;
        self.owns(snapshot)?;
        Ok(ReadBinding::new(
            Arc::clone(&database.source),
            Arc::clone(&self.opening),
        ))
    }
    pub(in crate::project_repository::sqlite_folder) async fn recheck(
        &self,
        snapshot: &FolderSnapshot,
    ) -> Result<()> {
        self.owns(snapshot)?;
        let mut database = self.database.lock().await;
        self.recheck_locked_scoped(&mut database, snapshot, snapshot.scope())
            .await
    }

    pub(in crate::project_repository::sqlite_folder) async fn read_history(
        &self,
        snapshot: &FolderSnapshot,
    ) -> Result<Vec<u8>> {
        self.owns(snapshot)?;
        let mut database = self.database.lock().await;
        let complete = self.capture_locked(&mut database).await?;
        if !snapshot.saved.same_saved_state(&complete) {
            return Err(changed());
        }
        self.owns(snapshot)?;
        complete.into_history().ok_or_else(changed)
    }

    /// Resolve a known attempted commit through the original opening. The old
    /// saved state may have advanced; its physical/opening authority must not.
    pub(in crate::project_repository::sqlite_folder) async fn resolve_commit(
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

    pub(in crate::project_repository::sqlite_folder) async fn read_asset(
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
        self.recheck_locked_scoped(&mut database, snapshot, snapshot.scope())
            .await?;
        let bytes = database.read_blob(hash, descriptor.size)?;
        self.recheck_locked_scoped(&mut database, snapshot, snapshot.scope())
            .await?;
        Ok(bytes)
    }
}
