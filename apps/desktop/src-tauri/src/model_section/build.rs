use super::{workspace::Workspace, SectionPlane, SectionReport};
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
    plane.normalized().map_err(|e| {
        AppError::Invalid(serde_json::json!({"code":e.code,"message":e.detail}).to_string())
    })?;
    crate::security::valid_id(project_id)?;
    let captured = capture::capture(state, project_id, expected_revision).await?;
    let hash = captured.hash.clone();
    let _permit = state
        .cad_tasks
        .acquire(project_id, "section", cancel.clone())
        .await?;
    if cancel.load(Ordering::Relaxed) {
        return Err(super::failure("SECTION_CANCELLED", "CAD section cancelled"));
    }
    capture::recheck(state, &captured).await?;
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
    crate::native::assets::stage_project_assets(
        state,
        &captured.snapshot.project,
        &document,
        &workspace.path,
    )?;
    capture::recheck(state, &captured).await?;
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
    capture::recheck(state, &captured).await?;
    if cancel.load(Ordering::Relaxed) {
        return Err(super::failure("SECTION_CANCELLED", "CAD section cancelled"));
    }
    Ok(report)
}
fn stale() -> AppError {
    AppError::Invalid(serde_json::json!({"code":"SECTION_STALE","message":"Project revision changed; recompute the section"}).to_string())
}
