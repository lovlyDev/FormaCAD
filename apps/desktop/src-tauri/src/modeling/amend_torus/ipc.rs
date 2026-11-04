use super::{LastTorusStatus, TorusReplacement};
use crate::{
    core::{AppState, Result},
    models::Project,
};
use tauri::State;

#[tauri::command]
pub async fn last_torus(
    project_id: String,
    state: State<'_, AppState>,
) -> Result<Option<LastTorusStatus>> {
    super::status(&state, &project_id).await
}

#[tauri::command]
pub async fn amend_last_torus(
    project_id: String,
    replacement: TorusReplacement,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<Project> {
    let request = super::prepare(&state, project_id, replacement).await?;
    Ok(super::super::publish(
        super::super::apply::apply(&state, request).await?,
        &app,
    ))
}
