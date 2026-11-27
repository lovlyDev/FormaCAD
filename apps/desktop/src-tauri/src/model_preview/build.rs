use super::{
    packet::{encode, PreviewMetrics},
    workspace::Workspace,
};
use crate::project_repository::backend::RepositoryRead;
use crate::{
    cad_ir::Document,
    core::{AppError, Result},
};
use sha2::{Digest, Sha256};
use std::{
    path::Path,
    sync::{atomic::AtomicBool, Arc},
};
pub fn validate_source(program: &str) -> Result<Document> {
    if program.is_empty() || program.len() > 60000 {
        return Err(AppError::Invalid("Model preview input is invalid".into()));
    }
    let document: Document = serde_json::from_str(program)?;
    document.validate().map_err(crate::cad_ir::app_error)?;
    Ok(document)
}
pub async fn build_with_executable(
    root: &Path,
    project_id: &str,
    program: &str,
    cancel: Arc<AtomicBool>,
    executable: &Path,
) -> Result<Vec<u8>> {
    build_staged(root, project_id, program, cancel, executable, None).await
}

pub async fn build_with_project(
    state: &crate::core::AppState,
    project: &crate::models::Project,
    program: &str,
    cancel: Arc<AtomicBool>,
    executable: &Path,
) -> Result<Vec<u8>> {
    let _cad = state
        .cad_tasks
        .acquire(&project.id, "model_preview", cancel.clone())
        .await?;
    build_staged(
        &state.root,
        &project.id,
        program,
        cancel,
        executable,
        Some(Assets::Managed(state, project)),
    )
    .await
}

enum Assets<'a, 'source> {
    Managed(&'a crate::core::AppState, &'a crate::models::Project),
    Read(&'a RepositoryRead<'source>),
}

pub async fn build_with_read(
    state: &crate::core::AppState,
    read: &RepositoryRead<'_>,
    program: &str,
    cancel: Arc<AtomicBool>,
    executable: &Path,
) -> Result<Vec<u8>> {
    let _cad = state
        .cad_tasks
        .acquire(read.task_scope_id(), "model_preview", cancel.clone())
        .await?;
    read.recheck().await?;
    build_staged(
        &state.root,
        &read.project().id,
        program,
        cancel,
        executable,
        Some(Assets::Read(read)),
    )
    .await
}

async fn build_staged(
    root: &Path,
    project_id: &str,
    program: &str,
    cancel: Arc<AtomicBool>,
    executable: &Path,
    assets: Option<Assets<'_, '_>>,
) -> Result<Vec<u8>> {
    let document = validate_source(program)?;
    let workspace = Workspace::create(root, project_id)?;
    match assets {
        Some(Assets::Managed(state, project)) => {
            crate::native::assets::stage_project_assets(state, project, &document, &workspace.path)?
        }
        Some(Assets::Read(read)) => {
            super::imports::stage_read(read, &document, &workspace.path).await?
        }
        None => {}
    }
    if !crate::native::worker::build_with_executable(program, &workspace.path, cancel, executable)
        .await?
    {
        return Err(AppError::Invalid("Model preview requires CAD IR v2".into()));
    }
    let metrics: PreviewMetrics =
        serde_json::from_slice(&std::fs::read(workspace.path.join("result.json"))?)?;
    let glb = std::fs::read(crate::security::guarded(
        &workspace.path,
        Path::new("preview.glb"),
    )?)?;
    encode(
        &format!("{:x}", Sha256::digest(program.as_bytes())),
        &metrics,
        &glb,
    )
}
