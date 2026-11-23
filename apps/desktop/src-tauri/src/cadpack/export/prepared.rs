use crate::{
    cadpack::{
        invalid, redaction, write_bundle_with, MAX_ARCHIVE_BYTES, MAX_FILES, MAX_PROJECT_BYTES,
    },
    core::Result,
    models::{Project, ProjectFile},
};
use std::{collections::HashMap, path::Path};

/// One exported document and archive with an upper bound on unique assets.
pub(super) struct BundlePlan<'a> {
    project: Project,
    history: &'a [u8],
}

impl<'a> BundlePlan<'a> {
    pub(super) fn new(project: &Project, history: &'a [u8], redact: bool) -> Result<Self> {
        let project = if redact {
            redaction::conversation_free(project)?
        } else {
            project.clone()
        };
        if project.files.len() > MAX_FILES {
            return Err(invalid("Project has too many attachments for a bundle"));
        }
        let project_size = serde_json::to_vec(&project)?.len() as u64;
        if project_size > MAX_PROJECT_BYTES {
            return Err(invalid("Project metadata exceeds bundle limit"));
        }
        let mut total = project_size + history.len() as u64;
        let mut seen = HashMap::new();
        for entry in &project.files {
            let hash = entry
                .sha256
                .as_deref()
                .ok_or_else(|| invalid("Project attachment checksum is missing"))?;
            if let Some(size) = seen.insert(hash, entry.size) {
                if size != entry.size {
                    return Err(invalid("Project attachment size mismatch"));
                }
                continue;
            }
            total = total
                .checked_add(entry.size)
                .ok_or_else(|| invalid("Project bundle exceeds 150 MB"))?;
        }
        if total > MAX_ARCHIVE_BYTES {
            return Err(invalid("Project bundle exceeds 150 MB"));
        }
        Ok(Self { project, history })
    }

    pub(super) fn attachments(&self) -> impl Iterator<Item = (&str, &ProjectFile)> {
        let mut seen = std::collections::HashSet::new();
        self.project.files.iter().filter_map(move |file| {
            let hash = file.sha256.as_deref()?;
            seen.insert(hash).then_some((hash, file))
        })
    }

    pub(super) fn write(
        &self,
        destination: &Path,
        mut attachments: HashMap<&str, Vec<u8>>,
    ) -> Result<()> {
        write_bundle_with(&self.project, destination, Some(self.history), |entry| {
            attachments
                .remove(entry.sha256.as_deref().unwrap_or_default())
                .ok_or_else(|| invalid("Project attachment checksum is missing"))
        })
    }
}
