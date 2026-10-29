//! Pure staged input validation shared by host clients and kernel workers.
use crate::{
    cad_ir::Document,
    model_measurement::schema::ImportedAssetSeal,
    native::{assets, NativeError},
};
use sha2::{Digest, Sha256};
use std::{io::Read, path::Path};

fn error(code: &str, detail: impl Into<String>) -> NativeError {
    NativeError {
        code: code.into(),
        detail: detail.into(),
    }
}

pub fn checked_bytes(cwd: &Path, relative: &Path, limit: usize) -> Result<Vec<u8>, NativeError> {
    let path = crate::security::guarded(cwd, relative)
        .map_err(|e| error("ASSET_INVALID", e.to_string()))?;
    let file = std::fs::File::open(path).map_err(|e| error("ASSET_MISSING", e.to_string()))?;
    let metadata = file
        .metadata()
        .map_err(|e| error("ASSET_INVALID", e.to_string()))?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > limit as u64 {
        return Err(error(
            "ASSET_LIMIT",
            "Staged measurement input size is invalid",
        ));
    }
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| error("ASSET_INVALID", e.to_string()))?;
    if bytes.is_empty() || bytes.len() > limit || bytes.len() as u64 != metadata.len() {
        return Err(error(
            "ASSET_LIMIT",
            "Staged measurement input changed size",
        ));
    }
    Ok(bytes)
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn imported_seals(
    document: &Document,
    cwd: &Path,
) -> Result<Vec<ImportedAssetSeal>, NativeError> {
    let references = crate::cad_ir::assets::references(document)
        .map_err(|e| error("ASSET_INVALID", e.to_string()))?;
    let mut total = 0usize;
    let mut seals = Vec::with_capacity(references.len());
    for (asset_id, sha256) in references {
        let bytes = checked_bytes(
            cwd,
            &assets::relative_path(sha256),
            crate::artifacts::MAX_FILE_BYTES,
        )?;
        total = total
            .checked_add(bytes.len())
            .ok_or_else(|| error("ASSET_LIMIT", "STEP size overflow"))?;
        if total > assets::MAX_TOTAL_STEP_BYTES {
            return Err(error(
                "ASSET_LIMIT",
                "Total imported STEP size exceeds limit",
            ));
        }
        if digest(&bytes) != sha256 {
            return Err(error(
                "ASSET_CHECKSUM_MISMATCH",
                "Imported STEP input differs from its seal",
            ));
        }
        seals.push(ImportedAssetSeal {
            asset_id: asset_id.into(),
            sha256: sha256.into(),
            size: bytes.len() as u64,
        });
    }
    Ok(seals)
}
