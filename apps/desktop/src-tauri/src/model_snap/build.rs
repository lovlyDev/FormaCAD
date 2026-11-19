//! Read-only exact snap against the committed authored body; never writes the live project.
use super::{client, schema::SnapQuery, ConfirmedSnap};
use crate::{
    core::{AppError, AppState, Result},
    model_measurement::{capture, workspace::Workspace},
};
use std::{
    path::Path,
    sync::{atomic::AtomicBool, Arc},
};

fn failure(code: &str, detail: &str) -> AppError {
    AppError::Invalid(serde_json::json!({"code":code,"message":detail}).to_string())
}

pub async fn snap_with_executable(
    state: &AppState,
    project_id: &str,
    expected_revision: &str,
    body_id: &str,
    query: &SnapQuery,
    executable: &Path,
) -> Result<ConfirmedSnap> {
    if !query.valid() {
        return Err(failure(
            "INVALID_SNAP_QUERY",
            "Snap reference kind or ray is invalid",
        ));
    }
    let cancel = Arc::new(AtomicBool::new(false));
    let captured = capture::capture(state, project_id, expected_revision, body_id).await?;
    let _permit = state
        .cad_tasks
        .acquire(project_id, "exact_snap", cancel.clone())
        .await?;
    capture::recheck(state, &captured).await?;
    let workspace = Workspace::create(&state.root, project_id)?;
    crate::artifacts::immutable_write(
        &workspace.path,
        Path::new("document.json"),
        &captured.program_bytes,
    )?;
    crate::artifacts::immutable_write(
        &workspace.path,
        Path::new("source.step"),
        &capture::source_bytes(state, &captured)?,
    )?;
    crate::native::assets::stage_project_assets(
        state,
        &captured.snapshot.project,
        &captured.document,
        &workspace.path,
    )?;
    let report = client::snap_with_executable(
        &workspace.path,
        cancel,
        executable,
        &captured.binding,
        query,
    )
    .await?;
    capture::recheck(state, &captured).await?;
    Ok(ConfirmedSnap {
        project_id: project_id.into(),
        report,
    })
}
