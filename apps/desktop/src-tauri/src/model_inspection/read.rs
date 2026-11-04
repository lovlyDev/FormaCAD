//! Pure committed snapshot inspection of any saved STEP revision.
use super::ExactModelProperties;
use crate::{
    core::{AppError, AppState, Result},
    native::inspection::ExactProperties,
    project_repository::backend::{ProjectRepository, RepositoryRead},
};
use std::{
    future::Future,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

pub(super) async fn inspect(
    state: &AppState,
    id: &str,
    revision: &str,
) -> Result<Option<ExactModelProperties>> {
    run(state, id, revision, |cwd, cancel| async move {
        crate::native::inspection::inspect_step(&cwd, cancel).await
    })
    .await
}

pub(super) async fn inspect_bound(
    state: &AppState,
    read: RepositoryRead<'_>,
    revision: &str,
) -> Result<Option<ExactModelProperties>> {
    run_bound(state, read, revision, |cwd, cancel| async move {
        crate::native::inspection::inspect_step(&cwd, cancel).await
    })
    .await
}

async fn run<F, Fut>(
    state: &AppState,
    id: &str,
    revision: &str,
    worker: F,
) -> Result<Option<ExactModelProperties>>
where
    F: FnOnce(PathBuf, Arc<AtomicBool>) -> Fut,
    Fut: Future<Output = Result<ExactProperties>>,
{
    let read = RepositoryRead::capture(ProjectRepository::managed(state, id)?).await?;
    run_bound(state, read, revision, worker).await
}

async fn run_bound<F, Fut>(
    state: &AppState,
    read: RepositoryRead<'_>,
    revision: &str,
    worker: F,
) -> Result<Option<ExactModelProperties>>
where
    F: FnOnce(PathBuf, Arc<AtomicBool>) -> Fut,
    Fut: Future<Output = Result<ExactProperties>>,
{
    let task_scope = read.task_scope_id().to_owned();
    let revision = read
        .project()
        .revisions
        .iter()
        .find(|r| r.id == revision)
        .ok_or_else(|| AppError::Invalid("CAD revision is missing".into()))?;
    let source = match revision.source.as_deref() {
        Some(source)
            if Path::new(source)
                .extension()
                .and_then(|v| v.to_str())
                .is_some_and(|v| {
                    v.eq_ignore_ascii_case("step") || v.eq_ignore_ascii_case("stp")
                }) =>
        {
            source
        }
        _ => {
            read.recheck().await?;
            return Ok(None);
        }
    };
    let file = read
        .project()
        .files
        .iter()
        .find(|f| f.name == source)
        .ok_or_else(|| AppError::Invalid("Saved STEP model is missing".into()))?;
    let bytes = checked_bytes(&read, file).await?;
    let cancel = Arc::new(AtomicBool::new(false));
    let _cad = state
        .cad_tasks
        .acquire(&task_scope, "inspection", cancel.clone())
        .await?;
    read.recheck().await?;
    let workspace = crate::project_access::transient::Workspace::create(&state.root)?;
    crate::artifacts::immutable_write(&workspace.path, Path::new("source.step"), &bytes)?;
    let properties = worker(workspace.path.clone(), cancel.clone()).await?;
    if cancel.load(Ordering::Relaxed) {
        return Err(AppError::Invalid("CAD_TASK_CANCELLED".into()));
    }
    if checked_bytes(&read, file).await? != bytes {
        return Err(AppError::Invalid("PROJECT_SNAPSHOT_CHANGED".into()));
    }
    read.recheck().await?;
    if cancel.load(Ordering::Relaxed) {
        return Err(AppError::Invalid("CAD_TASK_CANCELLED".into()));
    }
    Ok(Some(ExactModelProperties {
        volume_mm3: properties.volume_mm3,
        area_mm2: properties.area_mm2,
        face_count: properties.face_count,
        edge_count: properties.edge_count,
        bounds_mm: properties.bounds_mm,
    }))
}

async fn checked_bytes(
    read: &RepositoryRead<'_>,
    file: &crate::models::ProjectFile,
) -> Result<Vec<u8>> {
    read.read_asset(file).await.map_err(|error| match error {
        AppError::Invalid(code) if code == "Project file failed its integrity check" => {
            AppError::Invalid("PROJECT_SNAPSHOT_CHANGED".into())
        }
        error => error,
    })
}

// This fixture constructs STEP directly with OCCT; client-only coverage uses worker IPC.
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
#[cfg(all(test, feature = "native-occt"))]
mod tests;
