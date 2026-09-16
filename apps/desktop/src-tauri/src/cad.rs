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
    state: State<'_, AppState>,
) -> Result<String> {
    parameters.validate()?;
    let project = crate::projects::get(&state, &project_id).await?;
    let current = project
        .revisions
        .iter()
        .find(|r| Some(&r.id) == project.current_revision.as_ref())
        .ok_or_else(|| AppError::Invalid("No revision is available".into()))?;
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
        let bytes = crate::artifacts::read(&state, &project_id, file)?;
        let cwd =
            crate::security::guarded(&state.root, Path::new(&format!("{project_id}/output")))?;
        let name = format!("model-{}.step", uuid::Uuid::new_v4());
        crate::artifacts::immutable_write(&cwd, Path::new(&name), &bytes)?;
        return Ok(cwd.join(name).display().to_string());
    }
    if current.source.is_some() || current.parameters != parameters || parameters.kind == "blank" {
        return Err(AppError::Invalid(
            "STEP export requires a native parametric revision".into(),
        ));
    }
    crate::permissions::consume(&state, &project_id, "export_file").await?;
    let python = processes::python(&state.root)
        .ok_or_else(|| AppError::Invalid("Python is required for STEP export".into()))?;
    let cwd = crate::security::guarded(&state.root, Path::new(&format!("{project_id}/output")))?;
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
        Arc::new(AtomicBool::new(false)),
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
    Ok(output.display().to_string())
}
