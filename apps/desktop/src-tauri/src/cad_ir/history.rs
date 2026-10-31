//! Read-only structural comparison of saved, typed project revisions.
use super::{diff_documents, Document, DocumentDiff};
use crate::{
    core::{AppError, AppState, Result},
    project_repository::backend::{ProjectRepository, RepositorySnapshot},
};
use tauri::State;

#[tauri::command]
pub async fn compare_ir_revisions(
    project_id: String,
    from_revision_id: String,
    to_revision_id: String,
    state: State<'_, AppState>,
) -> Result<Option<DocumentDiff>> {
    compare_saved(&state, &project_id, &from_revision_id, &to_revision_id).await
}

async fn compare_saved(
    state: &AppState,
    project_id: &str,
    from_revision_id: &str,
    to_revision_id: &str,
) -> Result<Option<DocumentDiff>> {
    let repository = ProjectRepository::managed(state, project_id)?;
    let snapshot = repository.capture().await.map_err(|error| match error {
        AppError::Invalid(ref code) if code == "PROJECT_SNAPSHOT_MISSING" => {
            AppError::Invalid("Project was not found".into())
        }
        other => other,
    })?;
    compare_captured(&repository, &snapshot, from_revision_id, to_revision_id).await
}

// One original snapshot supplies BOTH revisions. Tests can retain this actual
// authority across close/reopen without timers, IPC fields, or global hooks.
async fn compare_captured(
    repository: &ProjectRepository<'_>,
    snapshot: &RepositorySnapshot,
    from_revision_id: &str,
    to_revision_id: &str,
) -> Result<Option<DocumentDiff>> {
    let project = snapshot.project();
    let read = |id: &str| -> Result<Option<Document>> {
        let revision = project
            .revisions
            .iter()
            .find(|revision| revision.id == id)
            .ok_or_else(|| AppError::Invalid("CAD revision is missing".into()))?;
        let Some(source) = revision.program.as_deref() else {
            return Ok(None);
        };
        if !source.trim_start().starts_with('{') {
            return Ok(None);
        }
        let value: serde_json::Value = serde_json::from_str(source)?;
        if value.get("schemaVersion").is_none() {
            return Ok(None);
        }
        let document: Document = serde_json::from_value(value)?;
        if document.revision_id != id {
            return Err(AppError::Invalid(
                "CAD IR revision does not match project".into(),
            ));
        }
        Ok(Some(document))
    };
    let result = match (read(from_revision_id)?, read(to_revision_id)?) {
        (Some(before), Some(after)) => diff_documents(&before, &after)
            .map(Some)
            .map_err(super::app_error),
        _ => Ok(None),
    }?;
    // Do not recapture a new token to authorize a delayed diff or legacy None.
    repository.recheck(snapshot).await?;
    Ok(result)
}

#[cfg(test)]
#[path = "history/session_tests.rs"]
mod session_tests;
