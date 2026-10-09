use crate::core::{AppError, Result};
use serde::{Deserialize, Serialize};

pub const STORAGE_VERSION: u32 = 2;
pub const MAX_METADATA: u64 = 32 * 1024 * 1024;
pub const MAX_TOTAL: u64 = 148 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Entry {
    pub path: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Manifest {
    pub storage_version: u32,
    pub project_id: String,
    pub generation: String,
    pub created_at: String,
    pub files: Vec<Entry>,
}

impl Manifest {
    pub fn validate(&self) -> Result<()> {
        crate::security::valid_id(&self.project_id)?;
        crate::security::valid_id(&self.generation)?;
        if self.storage_version != STORAGE_VERSION
            || self.files.is_empty()
            || self.files.len() > 10001
        {
            return Err(AppError::Invalid(
                "Unsupported project storage manifest".into(),
            ));
        }
        let mut names = std::collections::HashSet::new();
        let mut total = 0u64;
        for entry in &self.files {
            let valid_path = entry.path == "project.json"
                || entry.path == "history.json"
                || entry.path.strip_prefix("attachments/").is_some_and(|hash| {
                    crate::artifacts::valid_digest(hash) && hash == entry.sha256
                });
            total = total
                .checked_add(entry.size)
                .ok_or_else(|| AppError::Invalid("Project storage size overflow".into()))?;
            if !valid_path
                || !names.insert(&entry.path)
                || !crate::artifacts::valid_digest(&entry.sha256)
                || entry.size
                    > if entry.path == "project.json" {
                        MAX_METADATA
                    } else if entry.path == "history.json" {
                        16 * 1024 * 1024
                    } else {
                        crate::artifacts::MAX_FILE_BYTES as u64
                    }
            {
                return Err(AppError::Invalid("Invalid project storage entry".into()));
            }
        }
        if !names.contains(&"project.json".to_string()) || total > MAX_TOTAL {
            return Err(AppError::Invalid(
                "Project storage manifest is incomplete or too large".into(),
            ));
        }
        Ok(())
    }
}
