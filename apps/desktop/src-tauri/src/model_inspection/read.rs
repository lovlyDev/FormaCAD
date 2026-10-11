//! Pure committed snapshot inspection of any saved STEP revision.
use super::ExactModelProperties;
use crate::{
    core::{AppError, AppState, Result},
    native::inspection::ExactProperties,
    project_repository::legacy,
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
    let snapshot = legacy::capture_project(state, id).await?;
    let revision = snapshot
        .project
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
            legacy::recheck(state, &snapshot).await?;
            return Ok(None);
        }
    };
    let file = snapshot
        .project
        .files
        .iter()
        .find(|f| f.name == source)
        .ok_or_else(|| AppError::Invalid("Saved STEP model is missing".into()))?;
    let bytes = checked_bytes(state, id, file)?;
    let cancel = Arc::new(AtomicBool::new(false));
    let _cad = state
        .cad_tasks
        .acquire(id, "inspection", cancel.clone())
        .await?;
    legacy::recheck(state, &snapshot).await?;
    let workspace = crate::project_access::transient::Workspace::create(&state.root)?;
    crate::artifacts::immutable_write(&workspace.path, Path::new("source.step"), &bytes)?;
    let properties = worker(workspace.path.clone(), cancel.clone()).await?;
    if cancel.load(Ordering::Relaxed) {
        return Err(AppError::Invalid("CAD_TASK_CANCELLED".into()));
    }
    if checked_bytes(state, id, file)? != bytes {
        return Err(AppError::Invalid("PROJECT_SNAPSHOT_CHANGED".into()));
    }
    // Full project fencing currently includes unrelated project metadata.
    legacy::recheck(state, &snapshot).await?;
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

fn checked_bytes(state: &AppState, id: &str, file: &crate::models::ProjectFile) -> Result<Vec<u8>> {
    let bytes = crate::artifacts::read(state, id, file)?;
    if bytes.len() as u64 != file.size
        || file
            .sha256
            .as_ref()
            .is_some_and(|hash| crate::artifacts::digest(&bytes) != *hash)
    {
        return Err(AppError::Invalid("PROJECT_SNAPSHOT_CHANGED".into()));
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests;
