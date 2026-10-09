use super::AccessStatus;
use crate::core::{AppError, AppState, Result};
use tauri::State;

#[tauri::command]
pub async fn acquire_project_access(
    project_id: String,
    state: State<'_, AppState>,
) -> Result<AccessStatus> {
    let _write = state.writes.lock().await;
    crate::projects::get(&state, &project_id).await?;
    state.project_access.acquire(&state.root, &project_id)
}
#[tauri::command]
pub async fn release_project_access(project_id: String, state: State<'_, AppState>) -> Result<()> {
    let _write = state.writes.lock().await;
    if state.tasks.lock().await.contains_key(&project_id) {
        return Err(AppError::Invalid("PROJECT_ACCESS_BUSY".into()));
    }
    state.project_access.release(&project_id)
}
#[tauri::command]
pub fn project_access_status(
    project_id: String,
    state: State<'_, AppState>,
) -> Result<AccessStatus> {
    state.project_access.status(&state.root, &project_id)
}

#[tauri::command]
pub async fn copy_project_for_editing(
    project_id: String,
    state: State<'_, AppState>,
) -> Result<crate::models::Project> {
    super::copy::copy_for_editing(&state, &project_id).await
}
