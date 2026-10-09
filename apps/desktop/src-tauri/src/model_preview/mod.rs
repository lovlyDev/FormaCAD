//! Exact staged model preview. No grants consumed, revisions committed or artifacts attached.
use crate::core::{AppError, AppState, Result};
use std::sync::atomic::Ordering;
#[cfg(feature = "native-occt")]
use std::sync::{atomic::AtomicBool, Arc};
use tauri::State;
#[cfg(feature = "native-occt")]
pub mod build;
#[cfg(feature = "native-occt")]
mod packet;
#[cfg(feature = "native-occt")]
mod workspace;
#[tauri::command]
pub async fn preview_model(
    project_id: String,
    program: String,
    expected_revision: Option<String>,
    state: State<'_, AppState>,
) -> Result<tauri::ipc::Response> {
    #[cfg(not(feature = "native-occt"))]
    {
        let _ = (project_id, program, expected_revision, state);
        Err(AppError::Invalid(
            "Model preview requires the native OpenCascade build".into(),
        ))
    }
    #[cfg(feature = "native-occt")]
    {
        build::validate_source(&program)?;
        let project = crate::projects::get(&state, &project_id).await?;
        if project.current_revision != expected_revision {
            return Err(AppError::Invalid(
                "Project changed; refresh the model preview".into(),
            ));
        }
        let cancel = Arc::new(AtomicBool::new(false));
        let marker = format!("preview:{project_id}");
        {
            let mut tasks = state.tasks.lock().await;
            if tasks.contains_key(&project_id) {
                return Err(AppError::Invalid(
                    "A project task is already running".into(),
                ));
            }
            tasks.insert(project_id.clone(), cancel.clone());
            tasks.insert(marker.clone(), cancel.clone());
        }
        let result = async {
            let executable = crate::native::worker::worker_executable()?;
            let packet =
                build::build_with_project(&state, &project, &program, cancel.clone(), &executable)
                    .await?;
            if cancel.load(Ordering::Relaxed) {
                return Err(AppError::Invalid("Model preview cancelled".into()));
            }
            if crate::projects::get(&state, &project_id)
                .await?
                .current_revision
                != expected_revision
            {
                return Err(AppError::Invalid(
                    "Project changed; refresh the model preview".into(),
                ));
            }
            Ok(tauri::ipc::Response::new(packet))
        }
        .await;
        let mut tasks = state.tasks.lock().await;
        tasks.remove(&project_id);
        tasks.remove(&marker);
        result
    }
}
#[tauri::command]
pub async fn cancel_model_preview(project_id: String, state: State<'_, AppState>) -> Result<()> {
    crate::security::valid_id(&project_id)?;
    if let Some(cancel) = state
        .tasks
        .lock()
        .await
        .get(&format!("preview:{project_id}"))
    {
        cancel.store(true, Ordering::Relaxed);
    }
    Ok(())
}
