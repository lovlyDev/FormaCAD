//! Verify the exact published receipt through the originally selected directory.
//! Reopening is a read operation, never authority to adopt a replacement target.
use super::{
    format,
    session::{FolderSession, FolderSnapshot},
};
use crate::{
    core::{AppError, Result},
    project_repository::location::LocationIdentity,
};
use std::{collections::HashSet, path::Path};

pub(super) fn recheck_location(location: &LocationIdentity, root: &Path) -> Result<()> {
    if location.recheck(root, Path::new(""))? {
        Ok(())
    } else {
        Err(AppError::Invalid("PROJECT_SNAPSHOT_CHANGED".into()))
    }
}

pub(super) async fn verify(
    root: &Path,
    location: &LocationIdentity,
    expected: &format::Snapshot,
) -> Result<(FolderSession, FolderSnapshot)> {
    recheck_location(location, root)?;
    let repository = FolderSession::open(root).await?;
    let captured = repository.capture().await?;
    recheck_location(location, root)?;
    if !captured.same_saved_state(expected) {
        return Err(AppError::Invalid("PROJECT_SNAPSHOT_CHANGED".into()));
    }
    let mut checked = HashSet::new();
    for file in &captured.project().files {
        if checked.insert(file.sha256.as_deref()) {
            repository.read_asset(&captured, file).await?;
        }
    }
    repository.recheck(&captured).await?;
    recheck_location(location, root)?;
    Ok((repository, captured))
}
