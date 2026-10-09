use super::{app_error, relative_path, MAX_TOTAL_STEP_BYTES};
use crate::{
    cad_ir::Document,
    core::{AppState, Result},
    models::Project,
};
use std::path::Path;

pub fn stage_project_assets(
    state: &AppState,
    project: &Project,
    document: &Document,
    cwd: &Path,
) -> Result<()> {
    document.validate().map_err(crate::cad_ir::app_error)?;
    let mut total = 0usize;
    let mut inputs = Vec::new();
    for (_, hash) in
        crate::cad_ir::assets::references(document).map_err(crate::cad_ir::app_error)?
    {
        let file = project
            .files
            .iter()
            .find(|file| {
                file.sha256.as_deref() == Some(hash)
                    && Path::new(&file.name)
                        .extension()
                        .and_then(|e| e.to_str())
                        .is_some_and(|extension| {
                            extension.eq_ignore_ascii_case("step")
                                || extension.eq_ignore_ascii_case("stp")
                        })
            })
            .ok_or_else(|| {
                app_error(
                    "ASSET_MISSING",
                    "Referenced STEP is absent from this project",
                )
            })?;
        if file.size == 0 || file.size > crate::artifacts::MAX_FILE_BYTES as u64 {
            return Err(app_error(
                "ASSET_LIMIT",
                "Project STEP exceeds input limits",
            ));
        }
        if file.data.is_none() {
            let extension = Path::new(&file.name)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap()
                .to_ascii_lowercase();
            let path = crate::security::guarded(
                &state.root,
                Path::new(&format!("{}/attachments/{hash}.{extension}", project.id)),
            )?;
            let size = std::fs::metadata(path)
                .map_err(|e| app_error("ASSET_MISSING", e.to_string()))?
                .len();
            if size != file.size || size > crate::artifacts::MAX_FILE_BYTES as u64 {
                return Err(app_error(
                    "ASSET_LIMIT",
                    "Stored STEP size differs from project metadata",
                ));
            }
        }
        let bytes = crate::artifacts::read(state, &project.id, file)
            .map_err(|e| app_error("ASSET_CHECKSUM_MISMATCH", e.to_string()))?;
        if bytes.len() != file.size as usize || crate::artifacts::digest(&bytes) != hash {
            return Err(app_error(
                "ASSET_CHECKSUM_MISMATCH",
                "Project STEP failed content-address verification",
            ));
        }
        total = total
            .checked_add(bytes.len())
            .ok_or_else(|| app_error("ASSET_LIMIT", "STEP input size overflow"))?;
        if total > MAX_TOTAL_STEP_BYTES {
            return Err(app_error(
                "ASSET_LIMIT",
                "Total project STEP inputs exceed 128 MB",
            ));
        }
        inputs.push((relative_path(hash), bytes));
    }
    // Validate all references first, then materialize only immutable worker inputs.
    for (relative, bytes) in inputs {
        crate::artifacts::immutable_write(cwd, &relative, &bytes)?;
    }
    Ok(())
}
