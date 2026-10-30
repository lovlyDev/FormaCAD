//! Test-only access to the actual validated CADPACK writer and reader.
//! Source/session checks belong to the caller; destinations must be disposable
//! synthetic fixtures. This module adds no IPC, credentials, or app-index access.
use super::{
    history, invalid, read_bundle, write_bundle_with, MAX_ARCHIVE_BYTES, MAX_FILES,
    MAX_PROJECT_BYTES,
};
use crate::{
    core::Result,
    models::{Project, ProjectFile},
};
use std::{collections::BTreeMap, path::Path};

pub(crate) struct BundleProof {
    pub(crate) project_sha256: String,
    pub(crate) history_sha256: Option<String>,
    pub(crate) attachment_references: usize,
    pub(crate) unique_assets: usize,
    pub(crate) unique_asset_bytes: u64,
    pub(crate) asset_reader_calls: usize,
    pub(crate) archive_bytes: u64,
}

/// Write and read back the real package format. The callback must use bounded,
/// captured source reads; its descriptor supplies the exact permitted byte size.
/// No alternative ZIP codec or private BundleContents type escapes this module.
pub(crate) fn round_trip(
    project: &Project,
    destination: &Path,
    model_history: Option<&[u8]>,
    mut read_attachment: impl FnMut(&ProjectFile) -> Result<Vec<u8>>,
) -> Result<BundleProof> {
    project.validate()?;
    if project.files.len() > MAX_FILES || project.files.iter().any(|file| file.data.is_some()) {
        return Err(invalid(
            "Benchmark export requires normalized bounded package metadata",
        ));
    }
    let project_bytes = serde_json::to_vec(project)?;
    if project_bytes.len() as u64 > MAX_PROJECT_BYTES {
        return Err(invalid("Project metadata exceeds bundle limit"));
    }
    if let Some(bytes) = model_history {
        if bytes.len() as u64 > history::MAX_HISTORY_BYTES {
            return Err(invalid("HISTORY_ARCHIVE_TOO_LARGE"));
        }
        crate::project_history::validate_archive(project, bytes)?;
    }
    let mut expected = BTreeMap::new();
    let mut referenced_bytes = 0u64;
    for file in &project.files {
        let hash = file
            .sha256
            .as_ref()
            .filter(|hash| crate::artifacts::valid_digest(hash))
            .ok_or_else(|| invalid("Project attachment checksum is missing"))?;
        referenced_bytes = referenced_bytes
            .checked_add(file.size)
            .ok_or_else(|| invalid("Project attachments exceed 100 MB"))?;
        if referenced_bytes > 100 * 1024 * 1024 {
            return Err(invalid("Project attachments exceed 100 MB"));
        }
        if expected
            .insert(hash.clone(), file.size)
            .is_some_and(|size| size != file.size)
        {
            return Err(invalid("Project attachment size mismatch"));
        }
    }
    let unique_asset_bytes = expected.values().try_fold(0u64, |sum, &size| {
        sum.checked_add(size)
            .ok_or_else(|| invalid("Project bundle exceeds 150 MB"))
    })?;
    let total = (project_bytes.len() as u64)
        .checked_add(model_history.map_or(0, |bytes| bytes.len() as u64))
        .and_then(|size| size.checked_add(unique_asset_bytes))
        .ok_or_else(|| invalid("Project bundle exceeds 150 MB"))?;
    if total > MAX_ARCHIVE_BYTES {
        return Err(invalid("Project bundle exceeds 150 MB"));
    }
    let mut asset_reader_calls = 0usize;
    write_bundle_with(project, destination, model_history, |entry| {
        asset_reader_calls += 1;
        read_attachment(entry)
    })?;
    let (restored, assets, restored_history) = read_bundle(destination)?;
    if serde_json::to_vec(&restored)? != project_bytes
        || restored_history.as_deref() != model_history
        || asset_reader_calls != expected.len()
    {
        return Err(invalid("Benchmark bundle round-trip mismatch"));
    }
    let mut restored_assets = BTreeMap::new();
    for (hash, bytes) in assets {
        if crate::artifacts::digest(&bytes) != hash
            || restored_assets.insert(hash, bytes.len() as u64).is_some()
        {
            return Err(invalid("Benchmark bundle round-trip mismatch"));
        }
    }
    if restored_assets != expected {
        return Err(invalid("Benchmark bundle round-trip mismatch"));
    }
    Ok(BundleProof {
        project_sha256: crate::artifacts::digest(&project_bytes),
        history_sha256: model_history.map(crate::artifacts::digest),
        attachment_references: project.files.len(),
        unique_assets: expected.len(),
        unique_asset_bytes,
        asset_reader_calls,
        archive_bytes: std::fs::metadata(destination)?.len(),
    })
}

#[cfg(test)]
#[path = "benchmark_api_tests.rs"]
mod tests;
