//! Standalone folder creation: complete immutable assets precede the first SQL
//! reference. No app index, managed project row, UI grant or recovery is involved.
use super::{cas::PreparedCasBatch, format, write_connection::WriteConnection};
use crate::{
    core::{AppError, Result},
    models::Project,
    project_access::folder_lease::FolderLease,
};
use std::{path::Path, sync::Arc};

type BeforePublication = Box<dyn FnOnce() + Send>;

/// The selected root must already exist and be empty. Cancellation can leave
/// unreferenced files, never permission to overwrite or replay an operation.
pub(super) async fn create(
    root: &Path,
    project: Project,
    pending: Vec<(String, Vec<u8>)>,
    history: Option<Vec<u8>>,
) -> Result<format::CommitOutcome> {
    create_inner(root, project, pending, history, None).await
}

async fn create_inner(
    root: &Path,
    project: Project,
    pending: Vec<(String, Vec<u8>)>,
    history: Option<Vec<u8>>,
    before_publication: Option<BeforePublication>,
) -> Result<format::CommitOutcome> {
    // Fail malformed inputs before initialize_empty creates .forma/writer.lock.
    PreparedCasBatch::validate(&project, &pending)?;
    if let Some(bytes) = &history {
        crate::project_history::validate_archive(&project, bytes)?;
    }
    let root = std::path::absolute(root)?;
    let lease = FolderLease::initialize_empty(&root)?;
    let repository_id = loop {
        let candidate = uuid::Uuid::new_v4().to_string();
        if candidate != project.id {
            break candidate;
        }
    };
    // The blocking job owns every guard. Dropping its JoinHandle cannot revoke
    // the lease while publication is still running. SQL begins only after join.
    let (project, lease, publication) = tokio::task::spawn_blocking(move || {
        if let Some(barrier) = before_publication {
            barrier();
        }
        let publication =
            Arc::new(PreparedCasBatch::validate(&project, &pending)?.publish_new(&lease)?);
        Ok::<_, AppError>((project, lease, publication))
    })
    .await
    .map_err(|error| AppError::Io(std::io::Error::other(error)))??;

    let (writer, outcome) = WriteConnection::create(
        &root,
        lease,
        &repository_id,
        &project,
        history.as_deref(),
        publication,
    )
    .await?;
    // Even after COMMIT failure, discard the connection before attempting a
    // fresh outcome read. A close error cannot erase the known commit identity.
    match (outcome, writer.close().await) {
        (format::CommitOutcome::Committed(snapshot), Err(error)) => {
            Ok(format::CommitOutcome::Indeterminate {
                commit_id: snapshot.expectation().commit_id().to_owned(),
                error,
            })
        }
        (outcome, _) => Ok(outcome),
    }
}

#[cfg(test)]
mod tests;
