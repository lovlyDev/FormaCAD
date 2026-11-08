//! Trusted standalone publication. The host still owns modeling permissions.
use super::super::{
    cas::PreparedCasBatch, format, read_only::ReadOnlyDatabase, write_connection::WriteConnection,
};
use super::{changed, FolderSession, FolderSnapshot, ReadBinding};
use crate::{
    core::{AppError, Result},
    models::Project,
    project_access::folder_lease::FolderLease,
    project_history::PreparedHistory,
};
use std::sync::Arc;

impl FolderSession {
    pub(in crate::project_repository::sqlite_folder) async fn commit(
        &self,
        original: &FolderSnapshot,
        candidate: Project,
        pending: Vec<(String, Vec<u8>)>,
        transition: Option<PreparedHistory>,
    ) -> Result<format::CommitOutcome> {
        self.commit_inner(original, candidate, pending, transition, None)
            .await
    }

    async fn commit_inner(
        &self,
        original: &FolderSnapshot,
        candidate: Project,
        pending: Vec<(String, Vec<u8>)>,
        transition: Option<PreparedHistory>,
        before_publication: Option<Box<dyn FnOnce() + Send>>,
    ) -> Result<format::CommitOutcome> {
        self.owns(original)?;
        if candidate.id != self.project_id {
            return Err(changed());
        }
        PreparedCasBatch::validate(&candidate, &pending)?;
        crate::project_history::prepare_commit(
            Some(original.project()),
            Some(original.saved.history_state()),
            &candidate,
            transition.as_ref(),
        )?;
        // Keep this gate through SQL completion: other session reads must never
        // inspect our in-flight rollback journal with the pure-reader policy.
        let mut database = self.database.lock().await;
        self.recheck_locked(&mut database, original).await?;
        let lease = FolderLease::acquire(&self.root)?;
        let mut reader = ReadOnlyDatabase::open(&self.root).await?;
        self.recheck_locked(&mut database, original).await?;
        let binding = ReadBinding::new(Arc::clone(&database.source), Arc::clone(&self.opening));
        let job_binding = ReadBinding::new(Arc::clone(&database.source), Arc::clone(&self.opening));
        // Cancellation detaches this owned job. It cannot revoke the real lease
        // or CAS handles while filesystem IO is still executing.
        let (candidate, lease, published, mut reader) = tokio::task::spawn_blocking(move || {
            if let Some(barrier) = before_publication {
                barrier();
            }
            job_binding.check_active()?;
            let published = Arc::new(
                PreparedCasBatch::validate(&candidate, &pending)?.publish(&lease, &mut reader)?,
            );
            job_binding.check_active()?;
            Ok::<_, AppError>((candidate, lease, published, reader))
        })
        .await
        .map_err(|error| AppError::Io(std::io::Error::other(error)))??;
        self.recheck_locked(&mut database, original).await?;
        published.recheck(&mut reader)?;
        let mut writer = WriteConnection::open_for_publication(
            &self.root,
            lease,
            &original.saved,
            binding,
            Arc::clone(&published),
        )
        .await?;
        self.recheck_locked(&mut database, original).await?;
        let outcome = format::commit(
            writer.conn(),
            &original.saved,
            &candidate,
            transition.as_ref(),
        )
        .await;
        let outcome = match outcome {
            Ok(format::CommitOutcome::Committed(snapshot)) => {
                let commit_id = snapshot.expectation().commit_id().to_owned();
                let checked = async {
                    self.owns(original)?;
                    writer.recheck(&snapshot).await?;
                    self.owns(original)?;
                    // A confirmed COMMIT removed our journal. Full CAS checks
                    // through the independent safe reader are valid again.
                    published.recheck(&mut reader)?;
                    self.owns(original)
                }
                .await;
                match checked {
                    Ok(()) => format::CommitOutcome::Committed(snapshot),
                    Err(error) => format::CommitOutcome::Indeterminate { commit_id, error },
                }
            }
            Ok(other) => other,
            Err(error) => {
                let _ = writer.close().await;
                return Err(error);
            }
        };
        // Always discard a COMMIT-failed SQLx connection before outcome resolution.
        let close = writer.close().await;
        match (outcome, close) {
            (format::CommitOutcome::Committed(snapshot), Err(error)) => {
                Ok(format::CommitOutcome::Indeterminate {
                    commit_id: snapshot.expectation().commit_id().to_owned(),
                    error,
                })
            }
            (format::CommitOutcome::Committed(snapshot), Ok(())) => match self.owns(original) {
                Ok(()) => Ok(format::CommitOutcome::Committed(snapshot)),
                Err(error) => Ok(format::CommitOutcome::Indeterminate {
                    commit_id: snapshot.expectation().commit_id().to_owned(),
                    error,
                }),
            },
            (outcome, _) => Ok(outcome),
        }
    }
}

#[cfg(test)]
mod tests;
