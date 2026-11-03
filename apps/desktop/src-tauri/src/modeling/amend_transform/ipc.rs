use super::{LastTransformStatus, TransformReplacement};
use crate::{
    core::{AppState, Result},
    models::Project,
};
use tauri::State;

#[tauri::command]
pub async fn last_transform(
    project_id: String,
    state: State<'_, AppState>,
) -> Result<Option<LastTransformStatus>> {
    super::status(&state, &project_id).await
}

#[tauri::command]
pub async fn amend_last_transform(
    project_id: String,
    expected_revision: String,
    expected_generation: u64,
    transaction_id: String,
    replacements: Vec<TransformReplacement>,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<Project> {
    let request = super::prepare(
        &state,
        project_id,
        expected_revision,
        expected_generation,
        transaction_id,
        replacements,
    )
    .await?;
    Ok(super::super::publish(
        super::super::apply::apply(&state, request).await?,
        &app,
    ))
}
