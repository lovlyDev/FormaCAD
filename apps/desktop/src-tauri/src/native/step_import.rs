//! Convert an imported STEP using OCCT inside the isolated CAD worker.
use super::{NativeError, Solid};
use std::{fs, path::Path};

pub fn execute(cwd: &Path) -> Result<super::job::JobResult, NativeError> {
    let source = cwd.join("source.step");
    let size = fs::metadata(&source)
        .map_err(|error| NativeError {
            code: "STEP_IMPORT_FAILED".into(),
            detail: error.to_string(),
        })?
        .len();
    if size == 0 || size > 40 * 1024 * 1024 {
        return Err(NativeError {
            code: "STEP_SIZE_INVALID".into(),
            detail: "Imported STEP has invalid size".into(),
        });
    }
    let solid = Solid::read_step(&source)?;
    solid.write_glb(&cwd.join("preview.glb"))?;
    fs::copy(&source, cwd.join("model.step")).map_err(|error| NativeError {
        code: "STEP_IMPORT_FAILED".into(),
        detail: error.to_string(),
    })?;
    Ok(super::job::JobResult::from_solid(&solid))
}
