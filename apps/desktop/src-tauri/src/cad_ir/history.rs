//! Read-only structural comparison of saved, typed project revisions.
use super::{diff_documents, Document, DocumentDiff};
use crate::core::{AppError, AppState, Result};
use tauri::State;

#[tauri::command]
pub async fn compare_ir_revisions(
    project_id: String,
    from_revision_id: String,
    to_revision_id: String,
    state: State<'_, AppState>,
) -> Result<Option<DocumentDiff>> {
    let project = crate::projects::get(&state, &project_id).await?;
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
    match (read(&from_revision_id)?, read(&to_revision_id)?) {
        (Some(before), Some(after)) => diff_documents(&before, &after)
            .map(Some)
            .map_err(super::app_error),
        _ => Ok(None),
    }
}
