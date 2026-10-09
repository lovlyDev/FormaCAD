use super::{error, relative_path};
use crate::native::{NativeError, Solid};
use sha2::{Digest, Sha256};
use std::{
    io::Read,
    path::{Path, PathBuf},
};

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

fn open_input(path: &Path) -> Result<std::fs::File, NativeError> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(1); // FILE_SHARE_READ, without FILE_SHARE_WRITE or DELETE.
    }
    options
        .open(path)
        .map_err(|e| error("ASSET_MISSING", e.to_string()))
}

pub(super) fn checked_path(
    cwd: &Path,
    asset_id: &str,
    sha256: &str,
) -> Result<(PathBuf, usize), NativeError> {
    if !crate::cad_ir::assets::valid_reference(asset_id, sha256) {
        return Err(error(
            "ASSET_INVALID",
            "Invalid content-addressed STEP reference",
        ));
    }
    let path = crate::security::guarded(cwd, &relative_path(sha256))
        .map_err(|e| error("ASSET_INVALID", e.to_string()))?;
    let mut file = open_input(&path)?;
    let metadata = file
        .metadata()
        .map_err(|e| error("ASSET_INVALID", e.to_string()))?;
    if !metadata.is_file()
        || metadata.len() == 0
        || metadata.len() > crate::artifacts::MAX_FILE_BYTES as u64
    {
        return Err(error(
            "ASSET_LIMIT",
            "STEP input must be a bounded regular file",
        ));
    }
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 65536];
    let mut size = 0usize;
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|e| error("ASSET_INVALID", e.to_string()))?;
        if read == 0 {
            break;
        }
        size += read;
        if size > crate::artifacts::MAX_FILE_BYTES {
            return Err(error(
                "ASSET_LIMIT",
                "STEP input grew beyond its size limit",
            ));
        }
        digest.update(&buffer[..read]);
    }
    if format!("{:x}", digest.finalize()) != sha256 {
        return Err(error(
            "ASSET_CHECKSUM_MISMATCH",
            "STEP input checksum differs from the authored reference",
        ));
    }
    // The worker owns this staged directory. No path comes from the CAD document.
    Ok((path, size))
}
