//! Host-side snap worker client. The captured binding comes from the host lifecycle.
use super::{
    report::verify_report,
    schema::{SnapBinding, SnapQuery, SnapReport, MAX_REPORT_BYTES},
};
use crate::{
    core::{AppError, Result},
    native::reference_measurements::inputs::checked_bytes,
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
struct Identity {
    protocol_version: u32,
    request_id: String,
    status: String,
    code: Option<String>,
    detail: Option<String>,
    measurement_sha256: Option<String>,
}

fn invalid(detail: impl Into<String>) -> AppError {
    AppError::Invalid(
        serde_json::json!({"code":"SNAP_REPORT_INVALID","message":detail.into()}).to_string(),
    )
}

fn valid_code(code: &str) -> bool {
    !code.is_empty()
        && code.len() <= 80
        && code
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
}

pub async fn snap_with_executable(
    cwd: &Path,
    cancel: Arc<AtomicBool>,
    executable: &Path,
    binding: &SnapBinding,
    query: &SnapQuery,
) -> Result<SnapReport> {
    if !query.valid() || !binding.valid() {
        return Err(invalid("Snap reference or captured binding is invalid"));
    }
    let request_id = uuid::Uuid::new_v4().to_string();
    let input = serde_json::json!({"protocolVersion":1,"requestId":request_id,"operation":"snap_query",
        "bodyId":binding.body_id,"documentSha256":binding.document_sha256,"sourceSha256":binding.source_sha256,
        "sourceSize":binding.source_size,"snapQuery":query}).to_string();
    processes::run(
        &CommandSpec {
            executable: executable.into(),
            args: vec![],
            cwd: cwd.into(),
        },
        &input,
        cancel,
        Duration::from_secs(60),
    )
    .await?;
    let response =
        checked_bytes(cwd, Path::new("result.json"), 64 * 1024).map_err(|e| invalid(e.detail))?;
    let identity: Identity =
        serde_json::from_slice(&response).map_err(|e| invalid(e.to_string()))?;
    if identity.protocol_version != 1 || identity.request_id != request_id {
        return Err(invalid("Worker response identity mismatch"));
    }
    if identity.status == "error" {
        let code = identity.code.filter(|code| valid_code(code));
        let detail = identity.detail.filter(|detail| detail.len() <= 16 * 1024);
        if identity.measurement_sha256.is_some() || code.is_none() || detail.is_none() {
            return Err(invalid(
                "Failure response did not match the snap error contract",
            ));
        }
        return Err(AppError::Invalid(
            serde_json::json!({"code":code,"message":detail}).to_string(),
        ));
    }
    let report = checked_bytes(cwd, Path::new("snap.json"), MAX_REPORT_BYTES)
        .map_err(|e| invalid(e.detail))?;
    verify_report(&response, &report, &request_id, binding, query)
}
