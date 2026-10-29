//! Worker-only shape loading while retaining the checked input read lease.
use super::{
    error, relative_path,
    resolve::{checked_path, open_input},
};
use crate::native::{NativeError, Solid};
use std::path::Path;

pub fn read_step(cwd: &Path, asset_id: &str, sha256: &str) -> Result<Solid, NativeError> {
    if !crate::cad_ir::assets::valid_reference(asset_id, sha256) {
        return Err(error(
            "ASSET_INVALID",
            "Invalid content-addressed STEP reference",
        ));
    }
    let path = crate::security::guarded(cwd, &relative_path(sha256))
        .map_err(|e| error("ASSET_INVALID", e.to_string()))?;
    // Keep the verified source undeletable/unwritable while OCCT opens it on Windows.
    let lease = open_input(&path)?;
    let (path, _) = checked_path(cwd, asset_id, sha256)?;
    let solid = Solid::read_step(&path);
    drop(lease);
    solid
}
