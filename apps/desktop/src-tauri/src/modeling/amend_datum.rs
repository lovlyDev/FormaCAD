//! Amend a single appended construction feature through the regular geometry transaction.
use super::{amend_transform::read_document, apply::ApplyRequest};
use crate::{
    cad_ir::{Document, Operation},
    core::{AppError, AppState, Result},
    models::Project,
    project_history::AmendCapture,
};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DatumAdjustment {
    pub expected_revision: String,
    pub expected_generation: u64,
    pub transaction_id: String,
    pub operation: Operation,
}

fn stale() -> AppError {
    AppError::Invalid("AMEND_STALE".into())
}

fn definition(project: &Project, capture: &AmendCapture) -> Option<Document> {
    let current = read_document(project, &capture.expected_revision)?;
    let (last, prefix) = current.features.split_last()?;
    if last.suppressed || last.operation.datum_packet().is_none() {
        return None;
    }
    if let Some(revision) = &capture.base_revision {
        let before = read_document(project, revision)?;
        if before.features != prefix
            || before.parameters != current.parameters
            || before.bodies != current.bodies
        {
            return None;
        }
    } else if !prefix.is_empty() || !current.parameters.is_empty() || !current.bodies.is_empty() {
        return None;
    }
    Some(current)
}

pub async fn status(state: &AppState, project_id: &str) -> Result<Option<DatumAdjustment>> {
    let project = crate::projects::get(state, project_id).await?;
    let history = crate::project_history::load_state(&state.pool, &project).await?;
    let Some(capture) = history.capture_amend(&project)? else {
        return Ok(None);
    };
    Ok(definition(&project, &capture).and_then(|document| {
        Some(DatumAdjustment {
            expected_revision: capture.expected_revision,
            expected_generation: capture.generation,
            transaction_id: capture.transaction_id,
            operation: document.features.into_iter().last()?.operation,
        })
    }))
}

pub async fn prepare(
    state: &AppState,
    project_id: String,
    replacement: DatumAdjustment,
) -> Result<ApplyRequest> {
    let project = crate::projects::get(state, &project_id).await?;
    let history = crate::project_history::load_state(&state.pool, &project).await?;
    let capture = history.capture_amend(&project)?.ok_or_else(stale)?;
    if capture.expected_revision != replacement.expected_revision
        || capture.generation != replacement.expected_generation
        || capture.transaction_id != replacement.transaction_id
    {
        return Err(stale());
    }
    let mut next = definition(&project, &capture).ok_or_else(stale)?;
    let feature = next.features.last_mut().ok_or_else(stale)?;
    if std::mem::discriminant(&feature.operation) != std::mem::discriminant(&replacement.operation)
    {
        return Err(stale());
    }
    if feature.operation == replacement.operation {
        return Err(AppError::Invalid("NO_CHANGE".into()));
    }
    feature.operation = replacement.operation;
    next.validate().map_err(crate::cad_ir::app_error)?;
    let revision = uuid::Uuid::new_v4().to_string();
    next.revision_id = revision.clone();
    let mut request = ApplyRequest::new(
        project_id,
        serde_json::to_string(&next)?,
        format!("history:amend:{}", capture.expected_revision),
        Some(capture.expected_revision.clone()),
    );
    request.next_revision = Some(revision);
    request.history_amend = Some(capture);
    Ok(request)
}

#[tauri::command]
pub async fn last_datum(
    project_id: String,
    state: State<'_, AppState>,
) -> Result<Option<DatumAdjustment>> {
    status(&state, &project_id).await
}

#[tauri::command]
pub async fn amend_last_datum(
    project_id: String,
    replacement: DatumAdjustment,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<Project> {
    let request = prepare(&state, project_id, replacement).await?;
    Ok(super::publish(
        super::apply::apply(&state, request).await?,
        &app,
    ))
}
