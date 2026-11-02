//! Explicit conversion of a verified, backed-up published V2 generation.
//! Neither the original folder nor its backup is an in-place migration target.
use super::{cas::PreparedCasBatch, creation, format, session::FolderSession};
use crate::{
    core::{AppError, Result},
    project_repository::folder::{
        BackupProvenance, FolderReadSnapshot, FolderRepository, VerifiedV2Backup,
    },
};
use std::{collections::HashSet, path::Path};

pub(super) struct MigrationReport {
    pub backup: BackupProvenance,
    pub target: Result<format::CommitOutcome>,
}

/// A verified backup remains a real artifact even when target admission fails.
/// The report always retains its provenance; no replay or cleanup is implied.
pub(super) async fn migrate_verified_v2(
    source: &FolderRepository,
    original: &FolderReadSnapshot,
    backup: &VerifiedV2Backup,
    target: &Path,
) -> MigrationReport {
    migrate_inner(source, original, backup, target, None).await
}

async fn migrate_inner(
    source: &FolderRepository,
    original: &FolderReadSnapshot,
    backup: &VerifiedV2Backup,
    target: &Path,
    after_target: Option<Box<dyn FnOnce() + Send>>,
) -> MigrationReport {
    MigrationReport {
        backup: backup.provenance(),
        target: convert(source, original, backup, target, after_target).await,
    }
}

async fn convert(
    source: &FolderRepository,
    original: &FolderReadSnapshot,
    backup: &VerifiedV2Backup,
    target: &Path,
    after_target: Option<Box<dyn FnOnce() + Send>>,
) -> Result<format::CommitOutcome> {
    backup.recheck_source(source, original)?;
    backup.validate_target(target)?;
    backup.recheck()?;
    let project = backup.project().clone();
    // Complete metadata/size admission before loading any attachment buffers.
    PreparedCasBatch::validate(&project, &[])?;
    let history = backup.read_history()?;
    crate::project_history::validate_archive(&project, &history)?;
    let mut pending = Vec::new();
    let mut included = HashSet::new();
    for file in &project.files {
        let hash = file.sha256.as_deref().expect("validated CAS descriptor");
        if included.insert(hash) {
            pending.push((hash.to_owned(), backup.read_asset(file)?));
        }
    }
    backup.recheck_source(source, original)?;
    backup.recheck()?;
    let outcome = creation::create_checked(target, project, pending, Some(history), || async {
        backup.recheck_source(source, original)?;
        backup.recheck()
    })
    .await?;
    match outcome {
        format::CommitOutcome::Committed(saved) => {
            if let Some(after_target) = after_target {
                after_target();
            }
            match verify_target(source, original, backup, target).await {
                Ok(()) => Ok(format::CommitOutcome::Committed(saved)),
                Err(error) => Ok(format::CommitOutcome::Indeterminate {
                    commit_id: saved.expectation().commit_id().to_owned(),
                    error,
                }),
            }
        }
        outcome => Ok(outcome),
    }
}

async fn verify_target(
    source: &FolderRepository,
    original: &FolderReadSnapshot,
    backup: &VerifiedV2Backup,
    target: &Path,
) -> Result<()> {
    backup.recheck_source(source, original)?;
    backup.recheck()?;
    let repository = FolderSession::open(target).await?;
    let captured = repository.capture().await?;
    if serde_json::to_value(captured.project())? != serde_json::to_value(backup.project())?
        || repository.read_history(&captured).await? != backup.read_history()?
    {
        return Err(AppError::Invalid("PROJECT_SNAPSHOT_CHANGED".into()));
    }
    let mut checked = HashSet::new();
    for file in &captured.project().files {
        if checked.insert(file.sha256.as_deref()) {
            repository.read_asset(&captured, file).await?;
        }
    }
    repository.recheck(&captured).await?;
    backup.recheck_source(source, original)?;
    backup.recheck()
}

#[cfg(test)]
mod tests;
