use super::{failure, schema::FaceSectionQuery, workspace::Workspace, ConfirmedFaceSection};
use crate::project_repository::backend::RepositoryRead;
use crate::{
    core::{AppState, Result},
    model_measurement::capture,
};
use std::{
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

fn cancelled(token: &AtomicBool) -> Result<()> {
    if token.load(Ordering::Relaxed) {
        Err(failure("CAD_TASK_CANCELLED", "Face section cancelled"))
    } else {
        Ok(())
    }
}
#[allow(clippy::too_many_arguments)]
pub async fn section_with_executable(
    state: &AppState,
    project_id: &str,
    expected_revision: &str,
    body_id: &str,
    query: &FaceSectionQuery,
    cancel: Arc<AtomicBool>,
    executable: &Path,
) -> Result<ConfirmedFaceSection> {
    section_inner(
        state,
        project_id,
        expected_revision,
        body_id,
        query,
        cancel,
        executable,
        PhaseHook::default(),
    )
    .await
}
#[derive(Default)]
struct PhaseHook<'a> {
    #[cfg(test)]
    after_worker: Option<&'a (dyn Fn() + Send + Sync)>,
    _lifetime: std::marker::PhantomData<&'a ()>,
}
#[allow(clippy::too_many_arguments)]
async fn section_inner(
    state: &AppState,
    project_id: &str,
    expected_revision: &str,
    body_id: &str,
    query: &FaceSectionQuery,
    cancel: Arc<AtomicBool>,
    executable: &Path,
    hook: PhaseHook<'_>,
) -> Result<ConfirmedFaceSection> {
    section_inner_source(
        state,
        project_id,
        expected_revision,
        body_id,
        query,
        cancel,
        executable,
        hook,
        None,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn section_source_with_executable<'a>(
    state: &'a AppState,
    source: RepositoryRead<'a>,
    expected_revision: &str,
    body_id: &str,
    query: &FaceSectionQuery,
    cancel: Arc<AtomicBool>,
    executable: &Path,
) -> Result<ConfirmedFaceSection> {
    let project_id = source.project().id.clone();
    section_inner_source(
        state,
        &project_id,
        expected_revision,
        body_id,
        query,
        cancel,
        executable,
        PhaseHook::default(),
        Some(source),
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn section_inner_source<'a>(
    state: &'a AppState,
    project_id: &str,
    expected_revision: &str,
    body_id: &str,
    query: &FaceSectionQuery,
    cancel: Arc<AtomicBool>,
    executable: &Path,
    hook: PhaseHook<'_>,
    source: Option<RepositoryRead<'a>>,
) -> Result<ConfirmedFaceSection> {
    if !query.valid() {
        return Err(failure(
            "INVALID_SECTION_PLANE",
            "Face identity, signed offset or deflection is invalid",
        ));
    }
    cancelled(&cancel)?;
    let mut captured = match source {
        Some(read) => capture::capture_bound(read, expected_revision, body_id).await?,
        None => capture::capture(state, project_id, expected_revision, body_id).await?,
    };
    let task_scope = captured.snapshot.task_scope_id().to_owned();
    let _permit = state
        .cad_tasks
        .acquire(&task_scope, "face_section", cancel.clone())
        .await?;
    cancelled(&cancel)?;
    capture::recheck(&mut captured).await?;
    let workspace = Workspace::create(&state.root, project_id)?;
    crate::artifacts::immutable_write(
        &workspace.path,
        Path::new("document.json"),
        &captured.program_bytes,
    )?;
    crate::artifacts::immutable_write(
        &workspace.path,
        Path::new("source.step"),
        &capture::source_bytes(&captured).await?,
    )?;
    crate::model_measurement::staging::stage_assets(
        &captured.snapshot,
        &captured.document,
        &workspace.path,
    )
    .await?;
    cancelled(&cancel)?;
    let report = crate::native::face_section::client::section_with_executable(
        &workspace.path,
        cancel.clone(),
        executable,
        &captured.binding,
        query,
    )
    .await?;
    cancelled(&cancel)?;
    if !report.matches(&report.request_id, &captured.binding, query) {
        return Err(failure(
            "SECTION_FAILED",
            "Verified face section does not match captured inputs",
        ));
    }
    #[cfg(test)]
    if let Some(after_worker) = hook.after_worker {
        after_worker();
    }
    #[cfg(not(test))]
    let _ = hook;
    capture::recheck(&mut captured).await?;
    cancelled(&cancel)?;
    Ok(ConfirmedFaceSection {
        project_id: project_id.into(),
        report,
    })
}

#[cfg(test)]
#[path = "build_tests.rs"]
mod tests;
