#[cfg(not(feature = "native-occt"))]
use crate::processes::{self, CommandSpec};
use crate::{
    core::{AppError, AppState, Result},
    models::{Project, ProjectFile, Revision},
};
#[cfg(not(feature = "native-occt"))]
use sha2::{Digest, Sha256};
#[cfg(not(feature = "native-occt"))]
use std::time::Duration;
use std::{
    path::Path,
    sync::{atomic::AtomicBool, Arc},
};
use tauri::State;
#[cfg(feature = "native-occt")]
mod import_document;

#[tauri::command]
pub async fn convert_step(
    project_id: String,
    name: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<Project> {
    let _access = crate::project_access::ensure_write(&state, &project_id)?;
    let original = crate::projects::get(&state, &project_id).await?;
    let file = original
        .files
        .iter()
        .find(|f| f.name == name)
        .ok_or_else(|| AppError::Invalid("STEP attachment was not found".into()))?;
    if ![Some("step"), Some("stp")].contains(
        &Path::new(&name.to_lowercase())
            .extension()
            .and_then(|e| e.to_str()),
    ) {
        return Err(AppError::Invalid("Select a STEP attachment".into()));
    }
    crate::permissions::consume(&state, &project_id, "convert_file").await?;
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
        let cad_permit = state
            .cad_tasks
            .acquire(&project_id, "step_import", cancel.clone())
            .await?;
        let bytes = crate::artifacts::read(&state, &project_id, file)?;
        let job = uuid::Uuid::new_v4().to_string();
        #[cfg(feature = "native-occt")]
        let cwd = crate::security::guarded(
            &state.root,
            Path::new(&format!("{project_id}/cache/step-{job}")),
        )?;
        #[cfg(not(feature = "native-occt"))]
        let cwd = {
            let mut digest = Sha256::new();
            digest.update(&bytes);
            digest.update(include_bytes!("../scripts/convert_step.py"));
            let cache_key = format!("{:x}", digest.finalize());
            crate::security::guarded(
                &state.root,
                Path::new(&format!("{project_id}/cache/step-{cache_key}")),
            )?
        };
        std::fs::create_dir_all(&cwd)?;
        crate::artifacts::immutable_write(&cwd, Path::new("source.step"), &bytes)?;
        let output = crate::security::guarded(&cwd, Path::new("preview.glb"))?;
        #[cfg(feature = "native-occt")]
        crate::native::worker::import_step(&cwd, cancel.clone()).await?;
        #[cfg(not(feature = "native-occt"))]
        {
            let completed = crate::security::guarded(&cwd, Path::new("complete"))?;
            if !completed.is_file() || !output.is_file() {
                let python = processes::python(&state.root).ok_or_else(|| {
                    AppError::Invalid("Install the local CadQuery environment first".into())
                })?;
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
                    r#"{"source":"source.step","output":"preview.glb"}"#,
                    cancel.clone(),
                    Duration::from_secs(120),
                )
                .await?;
                // Mark complete only after a successful worker exit, never reuse partial output.
                crate::artifacts::immutable_write(&cwd, Path::new("complete"), b"1")?;
            }
        }
        if std::fs::metadata(&output)?.len() > crate::artifacts::MAX_FILE_BYTES as u64 {
            return Err(AppError::Invalid("Converted preview exceeds 40 MB".into()));
        }
        let preview = std::fs::read(output)?;
        drop(cad_permit);
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(AppError::Invalid("CAD_TASK_CANCELLED".into()));
        }
        let _lock = state.writes.lock().await;
        let mut project = crate::projects::get(&state, &project_id).await?;
        if project.current_revision != original.current_revision || project.files != original.files
        {
            return Err(AppError::Invalid(
                "Project changed during conversion; retry the import".into(),
            ));
        }
        let preview_name = format!("preview-{job}.glb");
        project.files.push(ProjectFile {
            name: preview_name.clone(),
            size: preview.len() as u64,
            kind: "model".into(),
            data: Some(crate::artifacts::data_url(&preview_name, &preview)),
            sha256: None,
        });
        let now = chrono::Utc::now().to_rfc3339();
        #[cfg(feature = "native-occt")]
        let imported_program = Some(import_document::source(&job, &name, &bytes)?);
        #[cfg(not(feature = "native-occt"))]
        let imported_program = None;
        project.revisions.push(Revision {
            program: imported_program,
            program_base: None,
            id: job.clone(),
            parent: project.current_revision.clone(),
            created_at: now.clone(),
            prompt: format!("Imported STEP: {name}"),
            source: Some(name),
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
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(AppError::Invalid("CAD_TASK_CANCELLED".into()));
        }
        crate::projects::persist(&state, project, pending, Some(&app)).await
    }
    .await;
    state.tasks.lock().await.remove(&project_id);
    result
}
