//! Exact whole-solid properties from the isolated OCCT worker.
use super::{job::JobResult, worker::worker_executable, NativeError, Solid};
use crate::{
    core::{AppError, Result},
    processes::{self, CommandSpec},
};
use serde::Deserialize;
use std::{
    path::Path,
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

pub fn inspect_source(cwd: &Path) -> std::result::Result<JobResult, NativeError> {
    let path = cwd.join("source.step");
    let size = std::fs::metadata(&path)
        .map_err(|error| NativeError {
            code: "STEP_IMPORT_FAILED".into(),
            detail: error.to_string(),
        })?
        .len();
    if size == 0 || size > 40 * 1024 * 1024 {
        return Err(NativeError {
            code: "STEP_SIZE_INVALID".into(),
            detail: "Inspection STEP has invalid size".into(),
        });
    }
    let solid = Solid::read_step(&path)?;
    Ok(JobResult::from_solid(&solid))
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExactProperties {
    pub volume_mm3: f64,
    pub area_mm2: f64,
    pub face_count: i32,
    pub edge_count: i32,
    pub bounds_mm: [f64; 3],
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InspectionResponse {
    protocol_version: u32,
    request_id: String,
    status: String,
    code: Option<String>,
    detail: Option<String>,
    volume_mm3: Option<f64>,
    area_mm2: Option<f64>,
    face_count: Option<i32>,
    edge_count: Option<i32>,
    bounds_mm: Option<[f64; 3]>,
}

pub async fn inspect_step(cwd: &Path, cancel: Arc<AtomicBool>) -> Result<ExactProperties> {
    let executable = worker_executable()?;
    inspect_with_executable(cwd, cancel, &executable).await
}

pub async fn inspect_with_executable(
    cwd: &Path,
    cancel: Arc<AtomicBool>,
    executable: &Path,
) -> Result<ExactProperties> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let input = serde_json::json!({
        "protocolVersion": 1,
        "requestId": request_id,
        "operation": "inspect_step"
    })
    .to_string();
    processes::run(
        &CommandSpec {
            executable: executable.to_path_buf(),
            args: Vec::new(),
            cwd: cwd.to_path_buf(),
        },
        &input,
        cancel,
        Duration::from_secs(120),
    )
    .await?;
    let response_path = crate::security::guarded(cwd, Path::new("result.json"))?;
    if std::fs::metadata(&response_path)?.len() > 64 * 1024 {
        return Err(AppError::Invalid(
            "CAD inspection response is too large".into(),
        ));
    }
    let response: InspectionResponse = serde_json::from_slice(&std::fs::read(response_path)?)?;
    if response.protocol_version != 1 || response.request_id != request_id {
        return Err(AppError::Invalid(
            "CAD inspection response did not match request".into(),
        ));
    }
    if response.status != "completed" {
        return Err(AppError::Invalid(
            serde_json::json!({"code":response.code,"message":response.detail}).to_string(),
        ));
    }
    let properties = ExactProperties {
        volume_mm3: response.volume_mm3.ok_or_else(missing_properties)?,
        area_mm2: response.area_mm2.ok_or_else(missing_properties)?,
        face_count: response.face_count.ok_or_else(missing_properties)?,
        edge_count: response.edge_count.ok_or_else(missing_properties)?,
        bounds_mm: response.bounds_mm.ok_or_else(missing_properties)?,
    };
    if !properties.volume_mm3.is_finite()
        || properties.volume_mm3 <= 0.0
        || !properties.area_mm2.is_finite()
        || properties.area_mm2 <= 0.0
        || properties.face_count <= 0
        || properties.edge_count <= 0
        || properties
            .bounds_mm
            .iter()
            .any(|value| !value.is_finite() || *value <= 0.0)
    {
        return Err(AppError::Invalid(
            "CAD inspection properties are invalid".into(),
        ));
    }
    Ok(properties)
}

fn missing_properties() -> AppError {
    AppError::Invalid("CAD inspection response has no properties".into())
}
