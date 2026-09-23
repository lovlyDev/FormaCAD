use crate::{
    core::{AppError, AppState, Result},
    models::{Project, ProjectFile, Revision},
    processes::{self, CommandSpec},
};
use std::{
    path::Path,
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};
use tauri::State;

#[tauri::command]
pub async fn apply_program(
    project_id: String,
    program: String,
    prompt: String,
    expected_revision: Option<String>,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<Project> {
    let original = crate::projects::get(&state, &project_id).await?;
    if program.trim().is_empty() || program.len() > 60000 || prompt.len() > 16000 {
        return Err(AppError::Invalid("Invalid modeling request".into()));
    }
    let document = if program.trim_start().starts_with('{') {
        Some(crate::cad_document::CadDocument::parse(&program)?.compile()?)
    } else {
        None
    };
    if original.current_revision != expected_revision {
        return Err(AppError::Invalid(
            "Project changed; regenerate the model from its current revision".into(),
        ));
    }
    let current = original
        .revisions
        .iter()
        .find(|r| Some(&r.id) == original.current_revision.as_ref());
    if current.and_then(|r| r.program.as_deref()) == Some(program.trim()) {
        return Ok(original);
    }
    crate::permissions::consume(&state, &project_id, "modify_project").await?;
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut tasks = state.tasks.lock().await;
        if tasks.contains_key(&project_id) {
            return Err(AppError::Invalid(
                "A project task is already running".into(),
            ));
        }
        tasks.insert(project_id.clone(), cancel.clone());
    }
    let result = async {
        let job = uuid::Uuid::new_v4().to_string();
        let cwd =
            crate::security::guarded(&state.root, Path::new(&format!("{project_id}/cache/{job}")))?;
        std::fs::create_dir_all(&cwd)?;
        let source = current
            .filter(|_| document.is_none())
            .and_then(|r| r.program_base.as_ref().or(r.source.as_ref()))
            .filter(|name| {
                name.to_lowercase().ends_with(".step") || name.to_lowercase().ends_with(".stp")
            });
        if let Some(name) = source {
            let file = original
                .files
                .iter()
                .find(|f| &f.name == name)
                .ok_or_else(|| AppError::Invalid("Base STEP is missing".into()))?;
            let bytes = crate::artifacts::read(&state, &project_id, file)?;
            crate::artifacts::immutable_write(&cwd, Path::new("base.step"), &bytes)?;
        }
        let python = processes::python(&state.root).ok_or_else(|| {
            AppError::Invalid("Install the local CadQuery environment first".into())
        })?;
        let build_output = processes::run(
            &CommandSpec {
                executable: python.clone(),
                args: vec![
                    "-I".into(),
                    "-c".into(),
                    include_str!("../scripts/model_program.py").into(),
                ],
                cwd: cwd.clone(),
            },
            &serde_json::json!({
                "program":document.as_ref().map(|d| d.source.as_str()).unwrap_or(&program),
                "features":document.as_ref().map(|d| &d.lines),
                "hasBase":source.is_some() && document.is_none()
            })
            .to_string(),
            cancel.clone(),
            Duration::from_secs(120),
        )
        .await?;
        let used_base =
            serde_json::from_str::<serde_json::Value>(build_output.trim())?["usesBase"] == true;
        processes::run(
            &CommandSpec {
                executable: python,
                args: vec![
                    "-I".into(),
                    "-c".into(),
                    include_str!("../scripts/convert_step.py").into(),
                ],
                cwd: cwd.clone(),
            },
            r#"{"source":"model.step","output":"preview.glb","parts":true}"#,
            cancel,
            Duration::from_secs(120),
        )
        .await?;
        let output = crate::security::guarded(&cwd, Path::new("preview.glb"))?;
        if std::fs::metadata(&output)?.len() > crate::artifacts::MAX_FILE_BYTES as u64 {
            return Err(AppError::Invalid("Converted preview exceeds 40 MB".into()));
        }
        let preview = std::fs::read(output)?;
        let _lock = state.writes.lock().await;
        let mut project = crate::projects::get(&state, &project_id).await?;
        if project.current_revision != original.current_revision || project.files != original.files
        {
            return Err(AppError::Invalid(
                "Project changed during modeling; retry the request".into(),
            ));
        }
        let step_path = crate::security::guarded(&cwd, Path::new("model.step"))?;
        if std::fs::metadata(&step_path)?.len() > crate::artifacts::MAX_FILE_BYTES as u64 {
            return Err(AppError::Invalid("Model STEP exceeds 40 MB".into()));
        }
        let step = std::fs::read(step_path)?;
        let step_name = format!("model-{job}.step");
        project.files.push(ProjectFile {
            name: step_name.clone(),
            size: step.len() as u64,
            kind: "model".into(),
            data: Some(crate::artifacts::data_url(&step_name, &step)),
            sha256: None,
        });
        let preview_name = format!("preview-{job}.glb");
        project.files.push(ProjectFile {
            name: preview_name.clone(),
            size: preview.len() as u64,
            kind: "model".into(),
            data: Some(crate::artifacts::data_url(&preview_name, &preview)),
            sha256: None,
        });
        let now = chrono::Utc::now().to_rfc3339();
        project.revisions.push(Revision {
            program: Some(program.trim().to_string()),
            program_base: if used_base { source.cloned() } else { None },
            id: job.clone(),
            parent: project.current_revision.clone(),
            created_at: now.clone(),
            prompt,
            source: Some(step_name),
            preview: Some(preview_name),
            parameters: crate::models::Parameters {
                kind: "blank".into(),
                width: 120.0,
                depth: 65.0,
                height: 60.0,
                thickness: 5.0,
                hole_diameter: 8.0,
                holes: 4,
            },
        });
        project.current_revision = Some(job);
        project.updated_at = now;
        let pending = crate::artifacts::normalize(&mut project)?;
        crate::projects::persist(&state, project, pending, Some(&app)).await
    }
    .await;
    state.tasks.lock().await.remove(&project_id);
    result
}
