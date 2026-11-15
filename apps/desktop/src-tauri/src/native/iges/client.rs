//! Host-side IGES worker client. No caller paths, executable or measured values in IPC.
use crate::{
    core::{AppError, Result},
    iges::schema::{Direction, ExchangeReport},
    processes::{self, CommandSpec},
};
use serde::Deserialize;
use std::{
    path::Path,
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Reply {
    protocol_version: u32,
    request_id: String,
    status: String,
    code: Option<String>,
    detail: Option<String>,
    step_sha256: Option<String>,
    preview_sha256: Option<String>,
    measurement_sha256: Option<String>,
}
pub struct Output {
    pub report: ExchangeReport,
    pub geometry: Vec<u8>,
    pub preview: Option<Vec<u8>>,
}
fn error(detail: &str) -> AppError {
    AppError::Invalid(
        serde_json::json!({"code":"IGES_REPORT_INVALID","message":detail}).to_string(),
    )
}
fn read(cwd: &Path, name: &str, limit: usize) -> Result<Vec<u8>> {
    let path = crate::security::guarded(cwd, Path::new(name))?;
    let meta = std::fs::symlink_metadata(&path)?;
    if !meta.is_file() || meta.len() == 0 || meta.len() > limit as u64 {
        return Err(error("Invalid exchange file size or type"));
    }
    Ok(std::fs::read(path)?)
}
pub async fn run(
    cwd: &Path,
    source: &[u8],
    direction: Direction,
    cancel: Arc<AtomicBool>,
    executable: &Path,
) -> Result<Output> {
    if source.is_empty() || source.len() > crate::artifacts::MAX_FILE_BYTES {
        return Err(error("Invalid input size"));
    }
    let request_id = uuid::Uuid::new_v4().to_string();
    let input_name = if direction == Direction::Import {
        "source.igs"
    } else {
        "source.step"
    };
    crate::artifacts::immutable_write(cwd, Path::new(input_name), source)?;
    let hash = crate::artifacts::digest(source);
    let request = serde_json::json!({"protocolVersion":1,"requestId":request_id,"operation":if direction==Direction::Import {"iges_import"} else {"iges_export"},"sourceSha256":hash,"sourceSize":source.len()});
    processes::run(
        &CommandSpec {
            executable: executable.into(),
            args: Vec::new(),
            cwd: cwd.into(),
        },
        &request.to_string(),
        cancel,
        Duration::from_secs(120),
    )
    .await?;
    let reply: Reply = serde_json::from_slice(&read(cwd, "result.json", 64 * 1024)?)?;
    if reply.protocol_version != 1 || reply.request_id != request_id {
        return Err(error("Worker identity mismatch"));
    }
    if reply.status != "completed" {
        return Err(AppError::Invalid(
            serde_json::json!({"code":reply.code,"message":reply.detail}).to_string(),
        ));
    }
    let raw = read(cwd, "iges-report.json", 64 * 1024)?;
    if reply.measurement_sha256.as_deref() != Some(crate::artifacts::digest(&raw).as_str()) {
        return Err(error("Exchange report checksum mismatch"));
    }
    let report: ExchangeReport = serde_json::from_slice(&raw)?;
    if !report.valid_for(&request_id, direction, &hash, source.len() as u64) {
        return Err(error("Invalid round-trip measurements or binding"));
    }
    let geometry = read(
        cwd,
        if direction == Direction::Import {
            "model.step"
        } else {
            "model.igs"
        },
        crate::artifacts::MAX_FILE_BYTES,
    )?;
    if geometry.len() as u64 != report.output_size
        || crate::artifacts::digest(&geometry) != report.output_sha256
    {
        return Err(error("Exchange output checksum mismatch"));
    }
    let preview = if direction == Direction::Import {
        let bytes = read(cwd, "preview.glb", crate::artifacts::MAX_FILE_BYTES)?;
        if report.preview_size != Some(bytes.len() as u64)
            || report.preview_sha256.as_deref() != Some(crate::artifacts::digest(&bytes).as_str())
            || reply.step_sha256.as_deref() != Some(report.output_sha256.as_str())
            || reply.preview_sha256 != report.preview_sha256
        {
            return Err(error("Import preview or STEP seal mismatch"));
        }
        Some(bytes)
    } else {
        if reply.step_sha256.is_some() || reply.preview_sha256.is_some() {
            return Err(error("Unexpected export artifacts"));
        }
        None
    };
    if read(cwd, input_name, crate::artifacts::MAX_FILE_BYTES)? != source {
        return Err(error("Staged input changed"));
    }
    Ok(Output {
        report,
        geometry,
        preview,
    })
}
