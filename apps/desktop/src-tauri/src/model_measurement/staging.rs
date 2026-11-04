//! Move verified source bytes into the existing native reference stager.
use super::failure;
use crate::{cad_ir::Document, core::Result, project_repository::backend::RepositoryRead};
use std::{
    collections::{HashMap, VecDeque},
    path::Path,
};

pub(crate) async fn stage_assets(
    read: &RepositoryRead<'_>,
    document: &Document,
    cwd: &Path,
) -> Result<()> {
    document.validate().map_err(crate::cad_ir::app_error)?;
    let mut inputs: HashMap<String, VecDeque<Vec<u8>>> = HashMap::new();
    let mut total = 0usize;
    for (_, hash) in
        crate::cad_ir::assets::references(document).map_err(crate::cad_ir::app_error)?
    {
        let file = read
            .project()
            .files
            .iter()
            .find(|file| {
                file.sha256.as_deref() == Some(hash)
                    && Path::new(&file.name)
                        .extension()
                        .and_then(|v| v.to_str())
                        .is_some_and(|v| {
                            v.eq_ignore_ascii_case("step") || v.eq_ignore_ascii_case("stp")
                        })
            })
            .ok_or_else(|| {
                failure(
                    "ASSET_MISSING",
                    "Referenced STEP is absent from this project",
                )
            })?;
        if file.size == 0 || file.size > crate::artifacts::MAX_FILE_BYTES as u64 {
            return Err(failure("ASSET_LIMIT", "Project STEP exceeds input limits"));
        }
        let bytes = read
            .read_asset(file)
            .await
            .map_err(super::snapshot::asset_error)?;
        total = total
            .checked_add(bytes.len())
            .ok_or_else(|| failure("ASSET_LIMIT", "STEP input size overflow"))?;
        if total > crate::native::assets::MAX_TOTAL_STEP_BYTES {
            return Err(failure(
                "ASSET_LIMIT",
                "Total project STEP inputs exceed 128 MB",
            ));
        }
        inputs.entry(hash.to_owned()).or_default().push_back(bytes);
    }
    // Move each buffer, including duplicate-reference occurrences. The native
    // stager retains its original validation and aggregate limit; no clone doubles RSS.
    crate::native::assets::stage_assets_with(read.project(), document, cwd, |file| {
        file.sha256
            .as_deref()
            .and_then(|hash| inputs.get_mut(hash))
            .and_then(VecDeque::pop_front)
            .ok_or_else(|| failure("ASSET_MISSING", "Captured STEP staging bytes are missing"))
    })?;
    read.recheck().await.map_err(super::snapshot::compatible)
}
