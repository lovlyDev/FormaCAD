//! Read-only exact snap against the committed authored body; never writes the live project.
use super::{client, schema::SnapQuery, ConfirmedSnap};
use crate::project_repository::backend::RepositoryRead;
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
    snap_inner(
        state,
        project_id,
        expected_revision,
        body_id,
        query,
        executable,
        None,
    )
    .await
}

pub async fn snap_source_with_executable<'a>(
    state: &'a AppState,
    source: RepositoryRead<'a>,
    expected_revision: &str,
    body_id: &str,
    query: &SnapQuery,
    executable: &Path,
) -> Result<ConfirmedSnap> {
    let project_id = source.project().id.clone();
    snap_inner(
        state,
        &project_id,
        expected_revision,
        body_id,
        query,
        executable,
        Some(source),
    )
    .await
}

async fn snap_inner<'a>(
    state: &'a AppState,
    project_id: &str,
    expected_revision: &str,
    body_id: &str,
    query: &SnapQuery,
    executable: &Path,
    source: Option<RepositoryRead<'a>>,
) -> Result<ConfirmedSnap> {
    if !query.valid() {
        return Err(failure(
            "INVALID_SNAP_QUERY",
            "Snap reference kind or ray is invalid",
        ));
    }
    let cancel = Arc::new(AtomicBool::new(false));
    let mut captured = match source {
        Some(read) => capture::capture_bound(read, expected_revision, body_id).await?,
        None => capture::capture(state, project_id, expected_revision, body_id).await?,
    };
    let task_scope = captured.snapshot.task_scope_id().to_owned();
    let _permit = state
        .cad_tasks
        .acquire(&task_scope, "exact_snap", cancel.clone())
        .await?;
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
    let report = client::snap_with_executable(
        &workspace.path,
        cancel,
        executable,
        &captured.binding,
        query,
    )
    .await?;
    capture::recheck(&mut captured).await?;
    Ok(ConfirmedSnap {
        project_id: project_id.into(),
        report,
    })
}
