//! Committed project authority and task lifecycle for read-only draft previews.
use crate::{
    core::{AppError, AppState, Result},
    models::Project,
    project_repository::legacy,
};
use std::{
    future::Future,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

pub(super) async fn preview(
    state: &AppState,
    id: &str,
    program: &str,
    expected: Option<String>,
) -> Result<Vec<u8>> {
    run(state, id, program, expected, |project, cancel| async move {
        let executable = crate::native::worker::worker_executable()?;
        super::build::build_with_project(state, &project, program, cancel, &executable).await
    })
    .await
}

async fn run<F, Fut>(
    state: &AppState,
    id: &str,
    program: &str,
    expected: Option<String>,
    worker: F,
) -> Result<Vec<u8>>
where
    F: FnOnce(Project, Arc<AtomicBool>) -> Fut,
    Fut: Future<Output = Result<Vec<u8>>>,
{
    let document = super::build::validate_source(program)?;
    let snapshot = legacy::capture_project(state, id).await?;
    if snapshot.project.current_revision != expected {
        return Err(AppError::Invalid(
            "Project changed; refresh the model preview".into(),
        ));
    }
    super::imports::verify(state, &snapshot.project, &document)?;
    let cancel = Arc::new(AtomicBool::new(false));
    let marker = format!("preview:{id}");
    {
        let mut tasks = state.tasks.lock().await;
        if tasks.contains_key(id) {
            return Err(AppError::Invalid(
                "A project task is already running".into(),
            ));
        }
        tasks.insert(id.into(), cancel.clone());
        tasks.insert(marker.clone(), cancel.clone());
    }
    let result = async {
        let packet = worker(snapshot.project.clone(), cancel.clone()).await?;
        if cancel.load(Ordering::Relaxed) {
            return Err(AppError::Invalid("Model preview cancelled".into()));
        }
        super::imports::verify(state, &snapshot.project, &document)?;
        // The full project seal deliberately also rejects metadata-only changes.
        legacy::recheck(state, &snapshot).await?;
        if cancel.load(Ordering::Relaxed) {
            return Err(AppError::Invalid("Model preview cancelled".into()));
        }
        Ok(packet)
    }
    .await;
    let mut tasks = state.tasks.lock().await;
    tasks.remove(id);
    tasks.remove(&marker);
    result
}

#[cfg(test)]
mod tests;
