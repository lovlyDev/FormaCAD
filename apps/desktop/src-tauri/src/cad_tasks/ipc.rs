use super::TaskInfo;
use crate::core::{AppState, Result};
use tauri::State;
#[tauri::command]
pub fn cad_task_status(state: State<'_, AppState>) -> Result<Vec<TaskInfo>> {
    state.cad_tasks.status()
}
#[tauri::command]
pub fn cancel_cad_task(task_id: String, state: State<'_, AppState>) -> Result<()> {
    state.cad_tasks.cancel(&task_id)
}
