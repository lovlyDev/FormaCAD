use crate::{
    core::{AppError, AppState, Result},
    models::Parameters,
    processes::{self, CommandSpec},
};
use std::{
    path::Path,
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};
use tauri::State;
#[tauri::command]
pub async fn export_step(
    project_id: String,
    parameters: Parameters,
    body_id: Option<String>,
    locale: Option<String>,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<Option<String>> {
    let project = crate::projects::get(&state, &project_id).await?;
    let filename = format!(
        "{}-r{}.step",
        project
            .name
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            })
            .collect::<String>(),
        project.revisions.len()
    );
    let bytes = generate_step(project_id, parameters, body_id, state).await?;
    crate::export_destination::save(app, filename, "step".into(), locale, bytes).await
}

async fn generate_step(
    project_id: String,
    parameters: Parameters,
    body_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<u8>> {
    parameters.validate()?;
    let project = crate::projects::get(&state, &project_id).await?;
    let current = project
        .revisions
        .iter()
        .find(|r| Some(&r.id) == project.current_revision.as_ref())
        .ok_or_else(|| AppError::Invalid("No revision is available".into()))?;
    if let Some(body_id) = body_id.as_deref() {
        return export_selected_body(&state, &project_id, current, body_id).await;
    }
    if let Some(source) = &current.source {
        if !source.to_lowercase().ends_with(".step") && !source.to_lowercase().ends_with(".stp") {
            return Err(AppError::Invalid(
                "This revision has no STEP geometry".into(),
            ));
        }
        crate::permissions::consume(&state, &project_id, "export_file").await?;
        let file = project
            .files
            .iter()
            .find(|f| &f.name == source)
            .ok_or_else(|| AppError::Invalid("STEP source is missing".into()))?;
        return crate::artifacts::read(&state, &project_id, file);
    }
    if current.source.is_some() || current.parameters != parameters || parameters.kind == "blank" {
        return Err(AppError::Invalid(
            "STEP export requires a native parametric revision".into(),
        ));
    }
    crate::permissions::consume(&state, &project_id, "export_file").await?;
    let python = processes::python(&state.root)
        .ok_or_else(|| AppError::Invalid("Python is required for STEP export".into()))?;
    let cancel = Arc::new(AtomicBool::new(false));
    let _cad = state
        .cad_tasks
        .acquire(&project_id, "step_export", cancel.clone())
        .await?;
    let workspace = crate::project_access::transient::Workspace::create(&state.root)?;
    let cwd = workspace.path.clone();
    let name = format!("model-{}.step", uuid::Uuid::new_v4());
    let script = include_str!("../scripts/export_step.py");
    let input = serde_json::json!({"parameters":parameters,"output":name});
    processes::run(
        &CommandSpec {
            executable: python,
            args: vec!["-I".into(), "-c".into(), script.into()],
            cwd: cwd.clone(),
        },
        &input.to_string(),
        cancel,
        Duration::from_secs(120),
    )
    .await?;
    let output = crate::security::guarded(&cwd, Path::new(&name))?;
    let metadata = std::fs::metadata(&output)?;
    if metadata.len() < 100 {
        return Err(AppError::Invalid(
            "CAD kernel returned an empty STEP artifact".into(),
        ));
    }
    Ok(std::fs::read(output)?)
}

#[cfg(feature = "native-occt")]
async fn export_selected_body(
    state: &AppState,
    project_id: &str,
    revision: &crate::models::Revision,
    body_id: &str,
) -> Result<Vec<u8>> {
    let program = revision
        .program
        .as_deref()
        .ok_or_else(|| AppError::Invalid("Current revision has no CAD IR".into()))?;
    let document: crate::cad_ir::Document = serde_json::from_str(program)?;
    document.validate().map_err(crate::cad_ir::app_error)?;
    if !document.bodies.iter().any(|body| body.id == body_id) {
        return Err(AppError::Invalid("Selected CAD body is missing".into()));
    }
    crate::permissions::consume(state, project_id, "export_file").await?;
    let cancel = Arc::new(AtomicBool::new(false));
    let _cad = state
        .cad_tasks
        .acquire(project_id, "step_export", cancel.clone())
        .await?;
    let workspace = crate::project_access::transient::Workspace::create(&state.root)?;
    let cwd = workspace.path.clone();
    let project = crate::projects::get(state, project_id).await?;
    if project.current_revision.as_deref() != Some(revision.id.as_str()) {
        return Err(AppError::Invalid(
            "Project changed; retry the CAD edit".into(),
        ));
    }
    crate::native::assets::stage_project_assets(state, &project, &document, &cwd)?;
    let built =
        crate::native::worker::build_selected_if_ir_v2(program, body_id, &cwd, cancel).await?;
    if !built {
        return Err(AppError::Invalid(
            "Current revision has no CAD IR v2".into(),
        ));
    }
    let bytes = std::fs::read(crate::security::guarded(&cwd, Path::new("model.step"))?)?;
    Ok(bytes)
}

#[cfg(not(feature = "native-occt"))]
async fn export_selected_body(
    _state: &AppState,
    _project_id: &str,
    _revision: &crate::models::Revision,
    _body_id: &str,
) -> Result<Vec<u8>> {
    Err(AppError::Invalid("Native CAD kernel is unavailable".into()))
}
