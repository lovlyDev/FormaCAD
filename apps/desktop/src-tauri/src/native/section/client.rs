use super::{app_error, SectionPlane, SectionReport, MAX_SECTION_BYTES};
use crate::{
    core::{AppError, Result},
    processes::{self, CommandSpec},
};
use serde::Deserialize;
use std::{
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Response {
    protocol_version: u32,
    request_id: String,
    status: String,
    code: Option<String>,
    detail: Option<String>,
    section_sha256: Option<String>,
}

pub async fn section_step(
    cwd: &Path,
    sha256: &str,
    plane: &SectionPlane,
    cancel: Arc<AtomicBool>,
) -> Result<SectionReport> {
    if cancel.load(Ordering::Relaxed) {
        return Err(failure("SECTION_CANCELLED", "CAD section cancelled"));
    }
    let executable = crate::native::worker::worker_executable()?;
    section_with_executable(cwd, sha256, plane, cancel, &executable).await
}
pub async fn section_with_executable(
    cwd: &Path,
    sha256: &str,
    plane: &SectionPlane,
    cancel: Arc<AtomicBool>,
    executable: &Path,
) -> Result<SectionReport> {
    if cancel.load(Ordering::Relaxed) {
        return Err(failure("SECTION_CANCELLED", "CAD section cancelled"));
    }
    plane.normalized().map_err(app_error)?;
    if !crate::artifacts::valid_digest(sha256) {
        return Err(failure("ASSET_INVALID", "Invalid section source checksum"));
    }
    let id = uuid::Uuid::new_v4().to_string();
    let input=serde_json::json!({"protocolVersion":1,"requestId":id,"operation":"section_step","sourceSha256":sha256,"sectionPlane":plane}).to_string();
    processes::run(
        &CommandSpec {
            executable: executable.to_path_buf(),
            args: vec![],
            cwd: cwd.to_path_buf(),
        },
        &input,
        cancel,
        Duration::from_secs(120),
    )
    .await?;
    let response_path = crate::security::guarded(cwd, Path::new("result.json"))?;
    if std::fs::metadata(&response_path)?.len() > 65536 {
        return Err(failure(
            "SECTION_FAILED",
            "Section worker response exceeded limit",
        ));
    }
    let response: Response = serde_json::from_slice(&std::fs::read(response_path)?)?;
    if response.protocol_version != 1 || response.request_id != id {
        return Err(failure(
            "SECTION_FAILED",
            "Section worker response did not match request",
        ));
    }
    if response.status != "completed" {
        return Err(AppError::Invalid(
            serde_json::json!({"code":response.code,"message":response.detail}).to_string(),
        ));
    }
    let path = crate::security::guarded(cwd, Path::new("section.json"))?;
    if std::fs::metadata(&path)?.len() > MAX_SECTION_BYTES as u64 {
        return Err(failure("SECTION_LIMIT", "Section report exceeded limit"));
    }
    let bytes = std::fs::read(path)?;
    if response.section_sha256.as_deref() != Some(crate::artifacts::digest(&bytes).as_str()) {
        return Err(failure(
            "ASSET_CHECKSUM_MISMATCH",
            "Section report checksum mismatch",
        ));
    }
    let report: SectionReport = serde_json::from_slice(&bytes)?;
    if report.source_sha256 != sha256 {
        return Err(failure(
            "SECTION_FAILED",
            "Section report source differs from request",
        ));
    }
    Ok(report)
}

fn failure(code: &str, detail: &str) -> AppError {
    app_error(super::error(code, detail))
}
