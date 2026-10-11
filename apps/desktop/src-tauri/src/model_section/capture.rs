//! Saved raw STEP input for manual sections; authored CAD IR is not required.
use crate::{
    core::{AppError, AppState, Result},
    models::ProjectFile,
    project_repository::legacy::{self, CommittedSnapshot},
};
use std::path::Path;

pub(super) struct CapturedSection {
    pub snapshot: CommittedSnapshot,
    pub hash: String,
    source: ProjectFile,
}

fn snapshot_error(error: AppError) -> AppError {
    match error {
        AppError::Invalid(ref code)
            if code == "PROJECT_SNAPSHOT_MISSING" || code == "PROJECT_SNAPSHOT_CHANGED" =>
        {
            super::stale()
        }
        other => other,
    }
}

pub(super) async fn capture(
    state: &AppState,
    project_id: &str,
    expected_revision: &str,
) -> Result<CapturedSection> {
    let snapshot = legacy::capture_project(state, project_id)
        .await
        .map_err(snapshot_error)?;
    let project = &snapshot.project;
    if project.current_revision.as_deref() != Some(expected_revision) {
        return Err(super::stale());
    }
    let revision = project
        .revisions
        .iter()
        .find(|revision| revision.id == expected_revision)
        .ok_or_else(super::stale)?;
    let name = revision.source.as_ref().ok_or_else(|| {
        super::super::failure(
            "ASSET_MISSING",
            "Current revision has no exact CAD source for section",
        )
    })?;
    let source = project
        .files
        .iter()
        .find(|file| &file.name == name)
        .cloned()
        .ok_or_else(|| {
            super::super::failure("ASSET_MISSING", "CAD section source attachment is missing")
        })?;
    if !Path::new(&source.name)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("step") || extension.eq_ignore_ascii_case("stp")
        })
    {
        return Err(super::super::failure(
            "ASSET_INVALID",
            "CAD section source is not STEP",
        ));
    }
    let hash = source
        .sha256
        .clone()
        .filter(|hash| crate::artifacts::valid_digest(hash))
        .ok_or_else(|| {
            super::super::failure("ASSET_INVALID", "CAD section source checksum is missing")
        })?;
    let captured = CapturedSection {
        snapshot,
        hash,
        source,
    };
    recheck(state, &captured).await?;
    Ok(captured)
}

/// Verify both repository identity and original bytes, not only the staged worker copy.
pub(super) async fn recheck(state: &AppState, captured: &CapturedSection) -> Result<()> {
    legacy::recheck(state, &captured.snapshot)
        .await
        .map_err(snapshot_error)?;
    if captured.source.size == 0 || captured.source.size > crate::artifacts::MAX_FILE_BYTES as u64 {
        return Err(super::super::failure(
            "ASSET_LIMIT",
            "CAD section source size is invalid",
        ));
    }
    let bytes = crate::artifacts::read(state, &captured.snapshot.project.id, &captured.source)
        .map_err(|error| super::super::failure("ASSET_CHECKSUM_MISMATCH", &error.to_string()))?;
    if bytes.len() as u64 != captured.source.size
        || crate::artifacts::digest(&bytes) != captured.hash
    {
        return Err(super::super::failure(
            "ASSET_CHECKSUM_MISMATCH",
            "CAD section source differs from its committed seal",
        ));
    }
    Ok(())
}
