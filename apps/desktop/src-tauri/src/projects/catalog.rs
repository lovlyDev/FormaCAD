//! IPC host policy for pure managed catalog and individual snapshot reads.
use crate::{
    core::{AppState, Result},
    models::Project,
    project_repository::{
        backend::ProjectRepository,
        catalog::{CatalogLimits, CatalogReport},
    },
};

// Response budgets, not peak-memory bounds. Enumeration never silently truncates.
const MAX_ENTRIES: usize = 1000;
const MAX_READY_PAYLOAD_BYTES: u64 = 128 * 1024 * 1024;

pub(super) async fn list(state: &AppState) -> Result<CatalogReport> {
    crate::project_repository::catalog::list_managed(
        state,
        CatalogLimits {
            max_entries: MAX_ENTRIES,
            max_total_payload_bytes: MAX_READY_PAYLOAD_BYTES,
        },
    )
    .await
}

pub(super) async fn read(state: &AppState, id: &str) -> Result<Project> {
    let repository = ProjectRepository::managed(state, id)?;
    let snapshot = repository.capture().await?;
    repository.recheck(&snapshot).await?;
    Ok(snapshot.project().clone())
}

#[cfg(test)]
mod tests;
