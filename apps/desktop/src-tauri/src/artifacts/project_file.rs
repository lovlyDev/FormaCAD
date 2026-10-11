//! Read-only attachment IPC: bounded verified bytes and committed metadata.
use crate::{
    core::{AppError, AppState, Result},
    project_repository::legacy::{self, CommittedSnapshot},
};

pub async fn read_for(state: &AppState, project_id: &str, name: &str) -> Result<String> {
    let snapshot = legacy::capture_project(state, project_id).await?;
    from_snapshot(state, &snapshot, name).await
}

async fn from_snapshot(
    state: &AppState,
    snapshot: &CommittedSnapshot,
    name: &str,
) -> Result<String> {
    let file = snapshot
        .project
        .files
        .iter()
        .find(|file| file.name == name)
        .ok_or_else(|| AppError::Invalid("Project attachment was not found".into()))?;
    let bytes = super::read(state, &snapshot.project.id, file)?;
    if bytes.len() as u64 != file.size
        || file
            .sha256
            .as_ref()
            .is_some_and(|hash| *hash != super::digest(&bytes))
    {
        return Err(AppError::Invalid(
            "Project file failed its integrity check".into(),
        ));
    }
    legacy::recheck(state, snapshot).await?;
    Ok(super::data_url(name, &bytes))
}

#[cfg(test)]
#[path = "project_file/tests.rs"]
mod tests;
