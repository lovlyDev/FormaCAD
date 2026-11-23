//! App-local editor text bound to an exact persisted FCP1 base.
use super::{FolderProject, ReadView};
use crate::{
    core::{AppError, Result},
    project_access::transient::Workspace,
};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use uuid::Uuid;

const MAX_DRAFT_BYTES: usize = 1024 * 1024;
const MAX_ENCODED_BYTES: u64 = 6 * MAX_DRAFT_BYTES as u64 + 4096;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DraftProvenance {
    repository_id: String,
    project_id: String,
    base_commit_id: String,
    base_revision_id: Option<String>,
}

impl DraftProvenance {
    fn capture(view: &ReadView) -> Self {
        Self {
            repository_id: view.snapshot.repository_id().into(),
            project_id: view.project().id.clone(),
            base_commit_id: view.commit_id().into(),
            base_revision_id: view.project().current_revision.clone(),
        }
    }

    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    pub fn base_commit_id(&self) -> &str {
        &self.base_commit_id
    }

    pub fn base_revision_id(&self) -> Option<&str> {
        self.base_revision_id.as_deref()
    }
}

/// One explicit host decision to retain editor text locally. It is not
/// serializable and cannot authorize a project write.
pub struct DraftApproval(DraftProvenance);

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LocalDraft {
    version: u8,
    id: String,
    provenance: DraftProvenance,
    editor_text: String,
}

impl LocalDraft {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn provenance(&self) -> &DraftProvenance {
        &self.provenance
    }

    pub fn editor_text(&self) -> &str {
        &self.editor_text
    }
}

/// The trusted host supplies the application's private local directory.
pub struct LocalDraftStore {
    root: PathBuf,
}

impl LocalDraftStore {
    pub fn open(app_local: &Path) -> Result<Self> {
        let root = crate::security::guarded(app_local, Path::new("folder-drafts"))?;
        fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    /// Save editor text even after disconnection using a previously approved
    /// base. Each save consumes one approval and creates one immutable record.
    pub fn save(&self, approval: DraftApproval, editor_text: String) -> Result<LocalDraft> {
        if editor_text.len() > MAX_DRAFT_BYTES {
            return Err(limit());
        }
        let draft = LocalDraft {
            version: 1,
            id: Uuid::new_v4().to_string(),
            provenance: approval.0,
            editor_text,
        };
        let workspace = Workspace::create(&self.root)?;
        let staging = workspace.path.join("draft");
        let mut file = File::create_new(&staging)?;
        file.write_all(&serde_json::to_vec(&draft)?)?;
        file.sync_all()?;
        drop(file);
        // A no-replace link exposes only complete, flushed records. Workspace
        // cleanup owns the temporary file, including on failure.
        fs::hard_link(staging, self.path(&draft.id)?)?;
        Ok(draft)
    }

    fn load(&self, id: &str, provenance: DraftProvenance) -> Result<LocalDraft> {
        let mut bytes = Vec::new();
        File::open(self.path(id)?)?
            .take(MAX_ENCODED_BYTES + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_ENCODED_BYTES {
            return Err(limit());
        }
        let draft: LocalDraft = serde_json::from_slice(&bytes)?;
        if draft.version != 1 || draft.id != id || draft.provenance != provenance {
            return Err(changed());
        }
        if draft.editor_text.len() > MAX_DRAFT_BYTES {
            return Err(limit());
        }
        Ok(draft)
    }

    fn path(&self, id: &str) -> Result<PathBuf> {
        crate::security::valid_id(id)?;
        crate::security::guarded(&self.root, Path::new(&format!("{id}.json")))
    }
}

impl FolderProject {
    /// Invoke only after the user explicitly chooses to keep a local editor
    /// draft. The approval is tied to this exact verified saved state.
    pub async fn approve_local_draft(&self, original: &ReadView) -> Result<DraftApproval> {
        self.recheck(original).await?;
        Ok(DraftApproval(DraftProvenance::capture(original)))
    }

    /// Return inert text for manual review on the same saved base. There is
    /// no stored permission, automatic apply or implicit fallback to another base.
    pub async fn read_local_draft(
        &self,
        original: &ReadView,
        store: &LocalDraftStore,
        id: &str,
    ) -> Result<LocalDraft> {
        self.recheck(original).await?;
        let draft = store.load(id, DraftProvenance::capture(original))?;
        self.recheck(original).await?;
        Ok(draft)
    }
}

fn changed() -> AppError {
    AppError::Invalid("PROJECT_SNAPSHOT_CHANGED".into())
}

fn limit() -> AppError {
    AppError::Invalid("PROJECT_FOLDER_LIMIT".into())
}

#[cfg(test)]
mod tests;
