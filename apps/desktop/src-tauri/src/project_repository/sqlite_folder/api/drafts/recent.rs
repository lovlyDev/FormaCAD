use super::{DraftProvenance, FolderProject, LocalDraft, LocalDraftStore, ReadView};
use crate::core::Result;
use std::{fs, io, path::PathBuf};

impl DraftProvenance {
    pub(super) fn relative_base(&self) -> Result<PathBuf> {
        for id in [&self.repository_id, &self.project_id, &self.base_commit_id] {
            crate::security::valid_id(id)?;
        }
        Ok(PathBuf::from("by-base")
            .join(&self.repository_id)
            .join(&self.project_id)
            .join(&self.base_commit_id))
    }
}

impl LocalDraftStore {
    fn latest(&self, provenance: DraftProvenance) -> Result<Option<LocalDraft>> {
        let entries = match fs::read_dir(self.base_directory(&provenance)?) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            other => other?,
        };
        let mut latest = None;
        for entry in entries {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let name = entry.file_name();
            let Some(id) = name.to_str().and_then(|name| name.strip_suffix(".json")) else {
                continue;
            };
            if crate::security::valid_id(id).is_err() {
                continue;
            }
            let candidate = (entry.metadata()?.modified()?, id.to_owned());
            if latest.as_ref().is_none_or(|current| &candidate > current) {
                latest = Some(candidate);
            }
        }
        latest.map(|(_, id)| self.load(&id, provenance)).transpose()
    }
}

impl FolderProject {
    /// Explicit draft review within the exact saved base, independently of
    /// the current opening ID. An unrelated local record is never considered.
    pub async fn latest_local_draft(
        &self,
        original: &ReadView,
        store: &LocalDraftStore,
    ) -> Result<Option<LocalDraft>> {
        self.recheck(original).await?;
        let draft = store.latest(DraftProvenance::capture(original))?;
        self.recheck(original).await?;
        Ok(draft)
    }
}
