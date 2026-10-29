//! Kernel-side whole STEP inspection.
use crate::native::{job::JobResult, NativeError, Solid};
use std::path::Path;

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
