//! Versioned, checksum-verified model history inside a portable project bundle.
use crate::{
    core::{AppError, Result},
    models::Project,
};
use serde::{Deserialize, Serialize};

pub(super) const MAX_HISTORY_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct HistoryEntry {
    size: u64,
    sha256: String,
}

pub(super) fn descriptor(bytes: &[u8]) -> Result<HistoryEntry> {
    if bytes.len() as u64 > MAX_HISTORY_BYTES {
        return Err(AppError::Invalid("HISTORY_ARCHIVE_TOO_LARGE".into()));
    }
    Ok(HistoryEntry {
        size: bytes.len() as u64,
        sha256: crate::artifacts::digest(bytes),
    })
}

pub(super) fn check(
    project: &Project,
    format_version: u32,
    descriptor: Option<&HistoryEntry>,
    bytes: Option<&[u8]>,
) -> Result<()> {
    match (format_version, descriptor, bytes) {
        (1, None, None) => Ok(()),
        (2, Some(entry), Some(bytes))
            if entry.size <= MAX_HISTORY_BYTES
                && entry.size == bytes.len() as u64
                && crate::artifacts::valid_digest(&entry.sha256)
                && crate::artifacts::digest(bytes) == entry.sha256 =>
        {
            crate::project_history::validate_archive(project, bytes)
        }
        _ => Err(AppError::Invalid("HISTORY_INVALID_ARCHIVE".into())),
    }
}
