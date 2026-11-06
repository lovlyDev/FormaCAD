use crate::core::{AppState, Result};
use serde::Serialize;
use tauri::State;

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ActivityEvent {
    pub id: i64,
    pub kind: String,
    pub to_revision: Option<String>,
    pub created_at: String,
}

pub async fn activity_for(state: &AppState, project_id: &str) -> Result<Vec<ActivityEvent>> {
    crate::security::valid_id(project_id)?;
    Ok(sqlx::query_as(
        "SELECT id,kind,to_revision,created_at FROM project_history_events
         WHERE project_id=? AND kind IN ('edit','undo','redo') ORDER BY id DESC LIMIT 200",
    )
    .bind(project_id)
    .fetch_all(&state.pool)
    .await?)
}

#[tauri::command]
pub async fn project_activity(
    project_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<ActivityEvent>> {
    activity_for(&state, &project_id).await
}
