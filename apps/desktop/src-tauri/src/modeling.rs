use crate::{
    core::{AppError, AppState, Result},
    models::Project,
    project_repository::window_sources::{
        write_ipc::{self, Command as FolderCommand, FolderWriteRequest, WriteReply},
        WindowSources,
    },
};
use tauri::{Emitter, State};
pub mod amend_datum;
pub mod amend_torus;
pub mod amend_transform;
pub mod apply;
mod build;
pub mod commands;
mod commit;
pub(crate) mod execution;
mod legacy;
pub(crate) mod prepared_change;
pub(crate) mod prepared_request;
mod program;
pub(crate) mod repository_apply;
pub(crate) use build::GeneratedModel;
mod workspace;

fn publish(outcome: apply::ApplyOutcome, app: &tauri::AppHandle) -> Project {
    if outcome.committed {
        let _ = app.emit("project://revision-created", &outcome.project.id);
    }
    outcome.project
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn apply_program(
    project_id: String,
    program: Option<String>,
    prompt: Option<String>,
    expected_revision: Option<String>,
    review_plan: Option<crate::agents::review_plan::ReviewPlan>,
    folder_write: Option<FolderWriteRequest>,
    window: tauri::WebviewWindow,
    sources: State<'_, WindowSources>,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<WriteReply> {
    if let Some(request) = folder_write {
        return write_ipc::execute(
            &sources,
            &state,
            window.label(),
            &project_id,
            request,
            FolderCommand::Program,
            program.is_some()
                || prompt.is_some()
                || expected_revision.is_some()
                || review_plan.is_some(),
        )
        .await;
    }
    let (Some(program), Some(prompt)) = (program, prompt) else {
        return Err(missing_payload());
    };
    let mut request = apply::ApplyRequest::new(project_id, program, prompt, expected_revision);
    request.review_plan = review_plan;
    Ok(publish(apply::apply(&state, request).await?, &app).into())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn apply_ir_commands(
    project_id: String,
    commands: Option<Vec<crate::cad_ir::Command>>,
    prompt: Option<String>,
    expected_revision: Option<String>,
    folder_write: Option<FolderWriteRequest>,
    window: tauri::WebviewWindow,
    sources: State<'_, WindowSources>,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<WriteReply> {
    if let Some(request) = folder_write {
        return write_ipc::execute(
            &sources,
            &state,
            window.label(),
            &project_id,
            request,
            FolderCommand::Commands,
            commands.is_some() || prompt.is_some() || expected_revision.is_some(),
        )
        .await;
    }
    let (Some(commands), Some(prompt)) = (commands, prompt) else {
        return Err(missing_payload());
    };
    let request =
        commands::prepare(&state, project_id, commands, prompt, expected_revision).await?;
    Ok(publish(apply::apply(&state, request).await?, &app).into())
}

fn missing_payload() -> AppError {
    AppError::Invalid("PROJECT_SNAPSHOT_CHANGED".into())
}
