//! Host-side worker client. Captured source binding is supplied by host lifecycle.
use crate::model_measurement::schema::MeasurementBinding;
use crate::{
    core::{AppError, Result},
    model_face_section::schema::{FaceSectionQuery, FaceSectionReport, MAX_FACE_SECTION_BYTES},
    processes::{self, CommandSpec},
};
use serde::Deserialize;
use std::{
    path::Path,
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Response {
    protocol_version: u32,
    request_id: String,
    status: String,
    code: Option<String>,
    detail: Option<String>,
    section_sha256: Option<String>,
    volume_mm3: Option<f64>,
    area_mm2: Option<f64>,
    face_count: Option<i32>,
    edge_count: Option<i32>,
    bounds_mm: Option<[f64; 3]>,
    step_sha256: Option<String>,
    preview_sha256: Option<String>,
    measurement_sha256: Option<String>,
}

fn invalid(detail: impl Into<String>) -> AppError {
    AppError::Invalid(
        serde_json::json!({"code":"SECTION_FAILED","message":detail.into()}).to_string(),
    )
}

pub fn verify_report(
    response: &[u8],
    report: &[u8],
    request_id: &str,
    binding: &MeasurementBinding,
    query: &FaceSectionQuery,
) -> Result<FaceSectionReport> {
    if response.is_empty()
        || response.len() > 64 * 1024
        || report.is_empty()
        || report.len() > MAX_FACE_SECTION_BYTES
    {
        return Err(invalid("Worker response or report exceeds bounds"));
    }
    let response: Response =
        serde_json::from_slice(response).map_err(|e| invalid(e.to_string()))?;
    if response.protocol_version != 1
        || response.request_id != request_id
        || response.status != "completed"
        || response.code.is_some()
        || response.detail.is_some()
        || response.volume_mm3.is_some()
        || response.area_mm2.is_some()
        || response.face_count.is_some()
        || response.edge_count.is_some()
        || response.bounds_mm.is_some()
        || response.step_sha256.is_some()
        || response.preview_sha256.is_some()
        || response.measurement_sha256.is_some()
        || response.section_sha256.as_deref() != Some(crate::artifacts::digest(report).as_str())
    {
        return Err(invalid(
            "Worker response did not exclusively match the face section request",
        ));
    }
    let report: FaceSectionReport =
        serde_json::from_slice(report).map_err(|e| invalid(e.to_string()))?;
    if !report.matches(request_id, binding, query) {
        return Err(invalid("Worker report echo or result is invalid"));
    }
    Ok(report)
}

pub async fn section_with_executable(
    cwd: &Path,
    cancel: Arc<AtomicBool>,
    executable: &Path,
    binding: &MeasurementBinding,
    query: &FaceSectionQuery,
) -> Result<FaceSectionReport> {
    if !query.valid() || !binding.valid() {
        return Err(invalid(
            "Measurement reference or captured binding is invalid",
        ));
    }
    let request_id = uuid::Uuid::new_v4().to_string();
    let input = serde_json::json!({"protocolVersion":1,"requestId":request_id,"operation":"section_reference",
        "bodyId":binding.body_id,"documentSha256":binding.document_sha256,"sourceSha256":binding.source_sha256,
        "sourceSize":binding.source_size,"faceSectionQuery":query}).to_string();
    processes::run(
        &CommandSpec {
            executable: executable.into(),
            args: vec![],
            cwd: cwd.into(),
        },
        &input,
        cancel,
        Duration::from_secs(120),
    )
    .await?;
    let response = crate::native::reference_measurements::execute::checked_bytes(
        cwd,
        Path::new("result.json"),
        64 * 1024,
    )
    .map_err(|e| invalid(e.detail))?;
    let decoded: Response =
        serde_json::from_slice(&response).map_err(|e| invalid(e.to_string()))?;
    if decoded.protocol_version != 1 || decoded.request_id != request_id {
        return Err(invalid("Worker response identity mismatch"));
    }
    if decoded.status == "error" {
        if decoded.section_sha256.is_some()
            || decoded.volume_mm3.is_some()
            || decoded.area_mm2.is_some()
            || decoded.face_count.is_some()
            || decoded.edge_count.is_some()
            || decoded.bounds_mm.is_some()
            || decoded.step_sha256.is_some()
            || decoded.preview_sha256.is_some()
            || decoded.measurement_sha256.is_some()
            || decoded.code.as_deref().is_none_or(|code| {
                code.is_empty()
                    || code.len() > 80
                    || !code
                        .bytes()
                        .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
            })
            || decoded
                .detail
                .as_ref()
                .is_none_or(|detail| detail.len() > 16 * 1024)
        {
            return Err(invalid(
                "Failure response did not match the face section error contract",
            ));
        }
        return Err(AppError::Invalid(
            serde_json::json!({"code":decoded.code,"message":decoded.detail}).to_string(),
        ));
    }
    let report = crate::native::reference_measurements::execute::checked_bytes(
        cwd,
        Path::new("face-section.json"),
        MAX_FACE_SECTION_BYTES,
    )
    .map_err(|e| invalid(e.detail))?;
    verify_report(&response, &report, &request_id, binding, query)
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod tests;
