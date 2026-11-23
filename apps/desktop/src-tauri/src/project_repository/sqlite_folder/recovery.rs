//! An explicit recovery publishes an independent copy. SQLite only replays
//! the captured journal inside app-owned scratch space.
use super::{
    cas::PreparedCasBatch,
    copy, creation, format, publication_verification,
    read_only::recovery_files::{Fingerprint, RecoveryFiles},
};
use crate::{
    core::{AppError, Result},
    project_access::transient::Workspace,
    project_repository::location::LocationIdentity,
};
use sqlx::{sqlite::SqliteConnectOptions, Connection, SqliteConnection};
use std::{collections::HashSet, fs, path::Path, sync::Arc};

struct Capture {
    source: RecoveryFiles,
    fingerprint: Fingerprint,
}

impl Capture {
    async fn recheck(self: Arc<Self>) -> Result<()> {
        tokio::task::spawn_blocking(move || self.source.recheck(&self.fingerprint))
            .await
            .map_err(join_error)?
    }
}

pub(super) async fn recover(
    source: &Path,
    destination: &Path,
    app_scratch: &Path,
) -> Result<format::CommitOutcome> {
    let canonical_source = fs::canonicalize(source)?;
    let canonical_destination = fs::canonicalize(destination)?;
    let scratch = fs::canonicalize(app_scratch)?;
    for (left, right) in [
        (&canonical_source, &canonical_destination),
        (&canonical_source, &scratch),
        (&canonical_destination, &scratch),
    ] {
        if left.starts_with(right) || right.starts_with(left) {
            return Err(changed());
        }
    }
    let destination_location = Arc::new(LocationIdentity::capture(destination, Path::new(""))?);
    let source = source.to_path_buf();
    let (capture, workspace) = tokio::task::spawn_blocking(move || stage(&source, &scratch))
        .await
        .map_err(join_error)??;
    recover_captured(capture, workspace, destination, destination_location).await
}

fn stage(source: &Path, scratch: &Path) -> Result<(Arc<Capture>, Workspace)> {
    let source = RecoveryFiles::open(source)?;
    let workspace = Workspace::create(scratch)?;
    let fingerprint = source.stage(&workspace.path)?;
    Ok((
        Arc::new(Capture {
            source,
            fingerprint,
        }),
        workspace,
    ))
}

async fn recover_captured(
    capture: Arc<Capture>,
    workspace: Workspace,
    destination: &Path,
    destination_location: Arc<LocationIdentity>,
) -> Result<format::CommitOutcome> {
    let recovered = recover_private(&workspace.path).await?;
    let history = recovered.history().ok_or_else(changed)?.to_vec();
    PreparedCasBatch::validate(recovered.project(), &[])?;
    let project = copy::fork_project(recovered.project(), &history)?;
    let assets_input = Arc::clone(&capture);
    let descriptors = project.files.clone();
    let pending = tokio::task::spawn_blocking(move || {
        let mut seen = HashSet::new();
        let mut pending = Vec::new();
        for file in descriptors {
            let hash = file.sha256.ok_or_else(changed)?;
            if seen.insert(hash.clone()) {
                let bytes = assets_input.source.read_blob(&hash, file.size)?;
                pending.push((hash, bytes));
            }
        }
        Ok::<_, AppError>(pending)
    })
    .await
    .map_err(join_error)??;
    drop(workspace);
    Arc::clone(&capture).recheck().await?;
    publication_verification::recheck_location(&destination_location, destination)?;

    let checked_source = Arc::clone(&capture);
    let checked_destination = Arc::clone(&destination_location);
    let outcome =
        creation::create_checked(destination, project, pending, Some(history), || async {
            checked_source.recheck().await?;
            publication_verification::recheck_location(&checked_destination, destination)
        })
        .await?;
    match outcome {
        format::CommitOutcome::Committed(saved) => {
            let verified = async {
                capture.recheck().await?;
                publication_verification::verify(destination, &destination_location, &saved).await
            }
            .await;
            match verified {
                Ok(_) => Ok(format::CommitOutcome::Committed(saved)),
                Err(error) => Ok(format::CommitOutcome::Indeterminate {
                    commit_id: saved.expectation().commit_id().to_owned(),
                    error,
                }),
            }
        }
        other => Ok(other),
    }
}

async fn recover_private(path: &Path) -> Result<format::Snapshot> {
    let options = SqliteConnectOptions::new()
        .filename(path.join("project.sqlite"))
        .create_if_missing(false)
        .busy_timeout(std::time::Duration::ZERO)
        .pragma("foreign_keys", "ON")
        .pragma("temp_store", "MEMORY")
        .pragma("trusted_schema", "OFF");
    let mut connection = SqliteConnection::connect_with(&options).await?;
    let recovered = format::read(&mut connection).await?;
    let integrity: String = sqlx::query_scalar("PRAGMA quick_check(1)")
        .fetch_one(&mut connection)
        .await?;
    if integrity != "ok" {
        return Err(changed());
    }
    connection.close().await?;
    Ok(recovered)
}

fn changed() -> AppError {
    AppError::Invalid("PROJECT_SNAPSHOT_CHANGED".into())
}

fn join_error(error: tokio::task::JoinError) -> AppError {
    AppError::Io(std::io::Error::other(error))
}

#[cfg(test)]
mod tests;
