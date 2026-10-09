use super::{error, SectionPlane, SectionReport, MAX_SECTION_BYTES};
use crate::native::NativeError;
use std::path::Path;

/// The only input is a host-staged, checksum-locked source inside this job.
pub fn execute_source(
    cwd: &Path,
    sha256: &str,
    plane: &SectionPlane,
) -> Result<SectionReport, NativeError> {
    plane.normalized()?;
    let solid = crate::native::assets::read_step(cwd, &format!("step_{sha256}"), sha256)?;
    let report = SectionReport {
        source_sha256: sha256.into(),
        geometry: solid.section(plane)?,
    };
    let bytes = serde_json::to_vec(&report).map_err(|e| error("SECTION_FAILED", e.to_string()))?;
    if bytes.len() > MAX_SECTION_BYTES {
        return Err(error("SECTION_LIMIT", "Section report exceeds 8 MB"));
    }
    let path = crate::security::guarded(cwd, Path::new("section.json"))
        .map_err(|e| error("SECTION_FAILED", e.to_string()))?;
    crate::artifacts::immutable_write(cwd, Path::new("section.json"), &bytes)
        .map_err(|e| error("SECTION_FAILED", e.to_string()))?;
    if !path.is_file() {
        return Err(error("SECTION_FAILED", "Section report was not published"));
    }
    Ok(report)
}
