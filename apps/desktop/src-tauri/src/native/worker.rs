//! App-side client for the isolated CAD worker protocol.
use crate::{
    cad_ir::Document,
    core::{AppError, Result},
    processes::{self, CommandSpec},
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    path::Path,
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkerResponse {
    protocol_version: u32,
    request_id: String,
    status: String,
    code: Option<String>,
    detail: Option<String>,
    step_sha256: Option<String>,
    preview_sha256: Option<String>,
}

fn verify_file(path: &Path, expected_sha256: Option<&str>) -> Result<()> {
    let len = std::fs::metadata(path)?.len();
    if len == 0 || len > crate::artifacts::MAX_FILE_BYTES as u64 {
        return Err(AppError::Invalid(
            "CAD worker output size is invalid".into(),
        ));
    }
    let bytes = std::fs::read(path)?;
    let actual = format!("{:x}", Sha256::digest(&bytes));
    if expected_sha256 != Some(actual.as_str()) {
        return Err(AppError::Invalid(
            "CAD worker output checksum mismatch".into(),
        ));
    }
    Ok(())
}

/// Returns true when the document was executed by the native worker.
pub async fn build_if_ir_v2(program: &str, cwd: &Path, cancel: Arc<AtomicBool>) -> Result<bool> {
    let executable = worker_executable()?;
    build_with_executable(program, cwd, cancel, &executable).await
}

pub async fn build_selected_if_ir_v2(
    program: &str,
    body_id: &str,
    cwd: &Path,
    cancel: Arc<AtomicBool>,
) -> Result<bool> {
    let executable = worker_executable()?;
    build_selected_with_executable(program, body_id, cwd, cancel, &executable).await
}

pub async fn build_selected_with_executable(
    program: &str,
    body_id: &str,
    cwd: &Path,
    cancel: Arc<AtomicBool>,
    executable: &Path,
) -> Result<bool> {
    build_document(program, Some(body_id), cwd, cancel, executable).await
}

/// Explicit executable injection is used by the protocol integration test.
pub async fn build_with_executable(
    program: &str,
    cwd: &Path,
    cancel: Arc<AtomicBool>,
    executable: &Path,
) -> Result<bool> {
    build_document(program, None, cwd, cancel, executable).await
}

async fn build_document(
    program: &str,
    body_id: Option<&str>,
    cwd: &Path,
    cancel: Arc<AtomicBool>,
    executable: &Path,
) -> Result<bool> {
    let value: serde_json::Value = match serde_json::from_str(program) {
        Ok(value) => value,
        Err(_) => return Ok(false),
    };
    if value
        .get("schemaVersion")
        .and_then(serde_json::Value::as_u64)
        != Some(2)
    {
        return Ok(false);
    }
    let document: Document = serde_json::from_value(value)?;
    document.validate().map_err(crate::cad_ir::app_error)?;
    if let Some(body_id) = body_id {
        if !document.bodies.iter().any(|body| body.id == body_id) {
            return Err(AppError::Invalid("Selected CAD body is missing".into()));
        }
    }
    let request_id = uuid::Uuid::new_v4().to_string();
    let input = serde_json::json!({
        "protocolVersion": 1,
        "requestId": request_id,
        "document": document,
        "bodyId": body_id
    })
    .to_string();
    run_request(&input, &request_id, cwd, cancel, executable).await?;
    Ok(true)
}

pub async fn import_step(cwd: &Path, cancel: Arc<AtomicBool>) -> Result<()> {
    let executable = worker_executable()?;
    import_step_with_executable(cwd, cancel, &executable).await
}

pub async fn import_step_with_executable(
    cwd: &Path,
    cancel: Arc<AtomicBool>,
    executable: &Path,
) -> Result<()> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let input = serde_json::json!({
        "protocolVersion": 1,
        "requestId": request_id,
        "operation": "import_step"
    })
    .to_string();
    run_request(&input, &request_id, cwd, cancel, executable).await
}

pub(crate) fn worker_executable() -> Result<std::path::PathBuf> {
    let current = std::env::current_exe()?;
    let parent = current
        .parent()
        .ok_or_else(|| AppError::Invalid("CAD worker path is unavailable".into()))?;
    Ok(parent.join(format!("forma-cad-worker{}", std::env::consts::EXE_SUFFIX)))
}

async fn run_request(
    input: &str,
    request_id: &str,
    cwd: &Path,
    cancel: Arc<AtomicBool>,
    executable: &Path,
) -> Result<()> {
    processes::run(
        &CommandSpec {
            executable: executable.to_path_buf(),
            args: Vec::new(),
            cwd: cwd.to_path_buf(),
        },
        input,
        cancel,
        Duration::from_secs(120),
    )
    .await?;
    let response_path = crate::security::guarded(cwd, Path::new("result.json"))?;
    if std::fs::metadata(&response_path)?.len() > 64 * 1024 {
        return Err(AppError::Invalid("CAD worker response is too large".into()));
    }
    let response: WorkerResponse = serde_json::from_slice(&std::fs::read(response_path)?)?;
    if response.protocol_version != 1 || response.request_id != request_id {
        return Err(AppError::Invalid(
            "CAD worker response did not match request".into(),
        ));
    }
    if response.status != "completed" {
        return Err(AppError::Invalid(
            serde_json::json!({"code":response.code,"message":response.detail}).to_string(),
        ));
    }
    verify_file(
        &crate::security::guarded(cwd, Path::new("model.step"))?,
        response.step_sha256.as_deref(),
    )?;
    verify_file(
        &crate::security::guarded(cwd, Path::new("preview.glb"))?,
        response.preview_sha256.as_deref(),
    )?;
    Ok(())
}
