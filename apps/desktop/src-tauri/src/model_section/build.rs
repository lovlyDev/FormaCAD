use super::{workspace::Workspace, SectionPlane, SectionReport};
use crate::project_repository::backend::RepositoryRead;
use crate::{
    cad_ir::{Body, Document, Feature, Operation},
    core::{AppError, AppState, Result},
};
use std::{
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
#[path = "capture.rs"]
mod capture;

pub async fn section_with_executable(
    state: &AppState,
    project_id: &str,
    expected_revision: &str,
    plane: &SectionPlane,
    cancel: Arc<AtomicBool>,
    executable: &Path,
) -> Result<SectionReport> {
    section_inner(
        state,
        project_id,
        expected_revision,
        plane,
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

async fn section_inner(
    state: &AppState,
    project_id: &str,
    expected_revision: &str,
    plane: &SectionPlane,
    cancel: Arc<AtomicBool>,
    executable: &Path,
    hook: PhaseHook<'_>,
) -> Result<SectionReport> {
    section_inner_source(
        state,
        project_id,
        expected_revision,
        plane,
        cancel,
        executable,
        hook,
        None,
    )
    .await
}

pub async fn section_source_with_executable<'a>(
    state: &'a AppState,
    source: RepositoryRead<'a>,
    expected_revision: &str,
    plane: &SectionPlane,
    cancel: Arc<AtomicBool>,
    executable: &Path,
) -> Result<SectionReport> {
    let id = source.project().id.clone();
    section_inner_source(
        state,
        &id,
        expected_revision,
        plane,
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
    plane: &SectionPlane,
    cancel: Arc<AtomicBool>,
    executable: &Path,
    hook: PhaseHook<'_>,
    source: Option<RepositoryRead<'a>>,
) -> Result<SectionReport> {
    plane.normalized().map_err(|e| {
        AppError::Invalid(serde_json::json!({"code":e.code,"message":e.detail}).to_string())
    })?;
    crate::security::valid_id(project_id)?;
    let captured = match source {
        Some(read) => capture::capture_bound(read, expected_revision).await?,
        None => capture::capture(state, project_id, expected_revision).await?,
    };
    let task_scope = captured.snapshot.task_scope_id().to_owned();
    let hash = captured.hash.clone();
    let _permit = state
        .cad_tasks
        .acquire(&task_scope, "section", cancel.clone())
        .await?;
    if cancel.load(Ordering::Relaxed) {
        return Err(super::failure("SECTION_CANCELLED", "CAD section cancelled"));
    }
    capture::recheck(&captured).await?;
    let workspace = Workspace::create(&state.root, project_id)?;
    let document = Document {
        schema_version: 2,
        revision_id: expected_revision.into(),
        parameters: vec![],
        features: vec![Feature {
            id: "section_source".into(),
            name: "Section source".into(),
            suppressed: false,
            operation: Operation::ImportStep {
                asset_id: format!("step_{hash}"),
                sha256: hash.clone(),
            },
        }],
        bodies: vec![Body {
            id: "section_body".into(),
            name: "Section body".into(),
            source_feature_id: "section_source".into(),
        }],
    };
    let mut bytes = Some(capture::bytes(&captured).await?);
    crate::native::assets::stage_assets_with(
        captured.snapshot.project(),
        &document,
        &workspace.path,
        |file| {
            if file.sha256.as_deref() != Some(hash.as_str()) {
                return Err(stale());
            }
            bytes.take().ok_or_else(stale)
        },
    )?;
    capture::recheck(&captured).await?;
    let report = crate::native::section::section_with_executable(
        &workspace.path,
        &hash,
        plane,
        cancel.clone(),
        executable,
    )
    .await?;
    if cancel.load(Ordering::Relaxed) {
        return Err(super::failure("SECTION_CANCELLED", "CAD section cancelled"));
    }
    #[cfg(test)]
    if let Some(after_worker) = hook.after_worker {
        after_worker();
    }
    #[cfg(not(test))]
    let _ = hook;
    capture::recheck(&captured).await?;
    if cancel.load(Ordering::Relaxed) {
        return Err(super::failure("SECTION_CANCELLED", "CAD section cancelled"));
    }
    Ok(report)
}
fn stale() -> AppError {
    AppError::Invalid(serde_json::json!({"code":"SECTION_STALE","message":"Project revision changed; recompute the section"}).to_string())
}

#[cfg(test)]
#[path = "build_tests.rs"]
mod tests;

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
#[path = "folder_build_tests.rs"]
mod folder_tests;
