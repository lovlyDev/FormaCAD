//! Explicit independent copy of one captured folder state. The source remains
//! read-only; destination CAS/SQL are published by the normal creation route.
use super::{
    cas::PreparedCasBatch,
    creation, format, publication_verification,
    session::{FolderSession, FolderSnapshot},
};
use crate::{core::Result, project_repository::location::LocationIdentity};
use std::{collections::HashSet, path::Path};

/// Trusted host API. A selected empty destination is required. Revision IDs and
/// portable history stay intact; project, repository and commit identity are new.
/// No credentials or process-local grants are manufactured by this operation.
pub(super) async fn save_copy(
    source: &FolderSession,
    original: &FolderSnapshot,
    destination: &Path,
) -> Result<format::CommitOutcome> {
    source.recheck(original).await?;
    // Validate every descriptor and the 40 MiB/file, 100 MiB total limits before
    // allocating attachment buffers. This performs no filesystem publication.
    PreparedCasBatch::validate(original.project(), &[])?;
    let history = source.read_history(original).await?;
    let mut pending = Vec::new();
    let mut seen = HashSet::new();
    for file in &original.project().files {
        // Validation above established normalized SHA-256 descriptors. Keep one
        // owned byte buffer per digest even when several filenames share it.
        let hash = file.sha256.as_deref().expect("validated CAS descriptor");
        if seen.insert(hash) {
            pending.push((hash.to_owned(), source.read_asset(original, file).await?));
        }
    }
    source.recheck(original).await?;
    let mut project = original.project().clone();
    project.id = loop {
        let id = uuid::Uuid::new_v4().to_string();
        if id != original.project().id {
            break id;
        }
    };
    project.created_at = chrono::Utc::now().to_rfc3339();
    project.updated_at = project.created_at.clone();
    crate::project_history::validate_archive(&project, &history)?;
    let location = LocationIdentity::capture(destination, Path::new(""))?;
    publication_verification::recheck_location(&location, destination)?;
    let outcome =
        creation::create_checked(destination, project, pending, Some(history), || async {
            source.recheck(original).await?;
            publication_verification::recheck_location(&location, destination)
        })
        .await?;
    // Creation owns its lease through actual connection close. If the source
    // changed while SQL was committing, never disguise the destination commit
    // as rollback or replay CAD/copy. The host can resolve this known UUID.
    match outcome {
        format::CommitOutcome::Committed(saved) => match async {
            source.recheck(original).await?;
            publication_verification::verify(destination, &location, &saved).await?;
            source.recheck(original).await
        }
        .await
        {
            Ok(()) => Ok(format::CommitOutcome::Committed(saved)),
            Err(error) => Ok(format::CommitOutcome::Indeterminate {
                commit_id: saved.expectation().commit_id().to_owned(),
                error,
            }),
        },
        outcome => Ok(outcome),
    }
}

#[cfg(test)]
mod tests;
