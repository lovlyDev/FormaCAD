use crate::{
    core::{AppState, Result},
    models::Project,
};
use tauri::{Emitter, State};
pub mod apply;
mod build;
pub mod commands;
mod commit;
mod legacy;
mod program;
mod workspace;

fn publish(outcome: apply::ApplyOutcome, app: &tauri::AppHandle) -> Project {
    if outcome.committed {
        let _ = app.emit("project://revision-created", &outcome.project.id);
    }
    outcome.project
}

#[tauri::command]
pub async fn apply_program(
    project_id: String,
    program: String,
    prompt: String,
    expected_revision: Option<String>,
    review_plan: Option<crate::agents::review_plan::ReviewPlan>,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<Project> {
    let mut request = apply::ApplyRequest::new(project_id, program, prompt, expected_revision);
    request.review_plan = review_plan;
    Ok(publish(apply::apply(&state, request).await?, &app))
}

#[tauri::command]
pub async fn apply_ir_commands(
    project_id: String,
    commands: Vec<crate::cad_ir::Command>,
    prompt: String,
    expected_revision: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<Project> {
    let request =
        commands::prepare(&state, project_id, commands, prompt, expected_revision).await?;
    Ok(publish(apply::apply(&state, request).await?, &app))
}
