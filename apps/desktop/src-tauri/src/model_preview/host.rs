//! Original-source authority and task lifecycle for read-only draft previews.
use crate::{
    core::{AppError, AppState, Result},
    project_repository::backend::{ProjectRepository, RepositoryRead},
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
    super::build::validate_source(program)?;
    let read = RepositoryRead::capture(ProjectRepository::managed(state, id)?).await?;
    preview_bound(state, read, program, expected).await
}

pub(super) async fn preview_bound<'a>(
    state: &'a AppState,
    read: RepositoryRead<'a>,
    program: &str,
    expected: Option<String>,
) -> Result<Vec<u8>> {
    run_bound(state, read, program, expected, |read, cancel| async move {
        let executable = crate::native::worker::worker_executable()?;
        super::build::build_with_read(state, &read, program, cancel, &executable).await
    })
    .await
}

// Preserve existing deterministic managed worker hooks over the shared host.
#[cfg(test)]
async fn run<F, Fut>(
    state: &AppState,
    id: &str,
    program: &str,
    expected: Option<String>,
    worker: F,
) -> Result<Vec<u8>>
where
    F: FnOnce(crate::models::Project, Arc<AtomicBool>) -> Fut,
    Fut: Future<Output = Result<Vec<u8>>>,
{
    super::build::validate_source(program)?;
    let read = RepositoryRead::capture(ProjectRepository::managed(state, id)?).await?;
    run_bound(state, read, program, expected, |read, cancel| {
        worker(read.project().clone(), cancel)
    })
    .await
}

async fn run_bound<'a, F, Fut>(
    state: &'a AppState,
    read: RepositoryRead<'a>,
    program: &str,
    expected: Option<String>,
    worker: F,
) -> Result<Vec<u8>>
where
    F: FnOnce(Arc<RepositoryRead<'a>>, Arc<AtomicBool>) -> Fut,
    Fut: Future<Output = Result<Vec<u8>>>,
{
    let document = super::build::validate_source(program)?;
    if read.project().current_revision != expected {
        return Err(AppError::Invalid(
            "Project changed; refresh the model preview".into(),
        ));
    }
    super::imports::verify_read(&read, &document).await?;
    let read = Arc::new(read);
    let cancel = Arc::new(AtomicBool::new(false));
    let scope = read.task_scope_id().to_owned();
    let marker = format!("preview:{scope}");
    {
        let mut tasks = state.tasks.lock().await;
        if tasks.contains_key(&scope) {
            return Err(AppError::Invalid(
                "A project task is already running".into(),
            ));
        }
        tasks.insert(scope.clone(), Arc::clone(&cancel));
        tasks.insert(marker.clone(), Arc::clone(&cancel));
    }
    let result = async {
        let packet = worker(Arc::clone(&read), Arc::clone(&cancel)).await?;
        if cancel.load(Ordering::Relaxed) {
            return Err(AppError::Invalid("Model preview cancelled".into()));
        }
        super::imports::verify_read(&read, &document).await?;
        // Preserve strict complete-snapshot semantics, including metadata changes.
        read.recheck().await?;
        if cancel.load(Ordering::Relaxed) {
            return Err(AppError::Invalid("Model preview cancelled".into()));
        }
        Ok(packet)
    }
    .await;
    complete(state, &scope, &marker, &read, &cancel, result).await
}

// Recheck AFTER releasing the task-map mutex: cleanup can outlive the source.
async fn complete(
    state: &AppState,
    scope: &str,
    marker: &str,
    read: &RepositoryRead<'_>,
    cancel: &AtomicBool,
    result: Result<Vec<u8>>,
) -> Result<Vec<u8>> {
    {
        let mut tasks = state.tasks.lock().await;
        tasks.remove(scope);
        tasks.remove(marker);
    }
    let packet = result?;
    read.recheck().await?;
    if cancel.load(Ordering::Relaxed) {
        return Err(AppError::Invalid("Model preview cancelled".into()));
    }
    Ok(packet)
}

#[cfg(all(
    test,
    feature = "native-occt",
    any(
        windows,
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        )
    )
))]
mod folder_tests;
#[cfg(test)]
mod tests;
