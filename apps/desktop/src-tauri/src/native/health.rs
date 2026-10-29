//! Bounded, nonmutating native-worker readiness handshake.
use crate::{
    core::{AppError, Result},
    processes::{self, CommandSpec},
};
use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

const MAX_HEALTH_BYTES: usize = 1024;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkerHealth {
    pub protocol_version: u32,
    pub status: String,
    pub engine: String,
    pub worker_version: String,
}

fn unavailable(detail: impl Into<String>) -> AppError {
    AppError::Invalid(
        serde_json::json!({"code":"NATIVE_KERNEL_UNAVAILABLE","message":detail.into()}).to_string(),
    )
}

fn validate(output: &str) -> Result<WorkerHealth> {
    if output.is_empty() || output.len() > MAX_HEALTH_BYTES {
        return Err(unavailable(
            "Native worker readiness response exceeds bounds",
        ));
    }
    let response: WorkerHealth =
        serde_json::from_str(output).map_err(|error| unavailable(error.to_string()))?;
    if response.protocol_version != 1
        || response.status != "ready"
        || response.engine != "occt"
        || response.worker_version != env!("CARGO_PKG_VERSION")
    {
        return Err(unavailable(
            "Native worker readiness response is incompatible",
        ));
    }
    Ok(response)
}

pub async fn probe(cancel: Arc<AtomicBool>) -> Result<WorkerHealth> {
    let executable =
        super::worker::worker_executable().map_err(|error| unavailable(error.to_string()))?;
    probe_isolated_with_executable(&executable, cancel).await
}

/// A stale worker may ignore CLI arguments and write legacy protocol artifacts.
/// Keep those writes outside both installation and project directories.
pub async fn probe_isolated_with_executable(
    executable: &Path,
    cancel: Arc<AtomicBool>,
) -> Result<WorkerHealth> {
    let executable = executable
        .canonicalize()
        .map_err(|error| unavailable(error.to_string()))?;
    let directory = tempfile::Builder::new()
        .prefix("forma-worker-health-")
        .tempdir()
        .map_err(|error| unavailable(error.to_string()))?;
    let result = probe_with_executable(&executable, directory.path(), cancel).await;
    directory
        .close()
        .map_err(|error| unavailable(format!("Native readiness cleanup failed: {error}")))?;
    result
}

pub async fn probe_with_executable(
    executable: &Path,
    cwd: &Path,
    cancel: Arc<AtomicBool>,
) -> Result<WorkerHealth> {
    probe_command(
        &CommandSpec {
            executable: executable.to_path_buf(),
            args: vec!["--health-check".into()],
            cwd: cwd.to_path_buf(),
        },
        cancel,
        Duration::from_secs(8),
    )
    .await
}

async fn probe_command(
    spec: &CommandSpec,
    cancel: Arc<AtomicBool>,
    timeout: Duration,
) -> Result<WorkerHealth> {
    if cancel.load(Ordering::Relaxed) {
        return Err(unavailable("Native readiness cancelled"));
    }
    let output = processes::run(spec, "", cancel.clone(), timeout)
        .await
        .map_err(|error| unavailable(error.to_string()))?;
    if cancel.load(Ordering::Relaxed) {
        return Err(unavailable("Native readiness cancelled"));
    }
    validate(&output)
}

/// Called only by the explicit worker CLI route. No workspace artifacts are created.
#[cfg(feature = "native-occt")]
pub fn kernel_readiness() -> std::result::Result<WorkerHealth, super::NativeError> {
    let solid = super::Solid::box_solid(1.0, 1.0, 1.0)?;
    if !solid.volume_mm3().is_finite()
        || !solid.surface_area_mm2().is_finite()
        || (solid.volume_mm3() - 1.0).abs() > 1e-9
        || (solid.surface_area_mm2() - 6.0).abs() > 1e-9
        || solid.face_count() != 6
        || solid.edge_count() != 12
        || solid
            .bounds_mm()
            .iter()
            .any(|value| !value.is_finite() || (*value - 1.0).abs() > 1e-9)
    {
        return Err(super::NativeError {
            code: "NATIVE_KERNEL_UNAVAILABLE".into(),
            detail: "Native readiness geometry failed its unit-box invariants".into(),
        });
    }
    Ok(WorkerHealth {
        protocol_version: 1,
        status: "ready".into(),
        engine: "occt".into(),
        worker_version: env!("CARGO_PKG_VERSION").into(),
    })
}

#[cfg(test)]
#[path = "health_tests.rs"]
mod tests;
