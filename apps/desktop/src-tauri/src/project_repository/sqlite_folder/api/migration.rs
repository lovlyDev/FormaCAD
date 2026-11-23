//! Host entry for explicit exact V2 backup before independent FCP1 conversion.
use super::super::migration as backend;
use super::Publication;
use crate::{
    core::Result,
    project_repository::folder::{
        BackupProvenance, FolderReadSnapshot, FolderRepository, VerifiedV2Backup,
    },
};
use std::path::{Path, PathBuf};

/// Descriptive provenance, not authority to write or migrate a source.
pub struct BackupRecord {
    pub root: PathBuf,
    pub manifest_sha256: String,
    pub generation: String,
    pub project_id: String,
}
impl From<BackupProvenance> for BackupRecord {
    fn from(provenance: BackupProvenance) -> Self {
        Self {
            root: provenance.root,
            manifest_sha256: provenance.manifest_sha256,
            generation: provenance.generation,
            project_id: provenance.project_id,
        }
    }
}

/// A completed and independently reread backup of the original physical opening.
/// No constructor accepts a serialized record, path alone or success flag.
pub struct V2Backup {
    proof: VerifiedV2Backup,
}
pub struct MigrationReport {
    pub backup: BackupRecord,
    pub target: Result<Publication>,
}
impl V2Backup {
    /// Blocking bounded IO. A UI host must schedule this on its blocking worker.
    /// The explicitly selected backup destination must already exist and be empty.
    pub fn create(
        source: &FolderRepository,
        original: &FolderReadSnapshot,
        destination: &Path,
    ) -> Result<Self> {
        Ok(Self {
            proof: source.backup_committed(original, destination)?,
        })
    }
    pub fn record(&self) -> BackupRecord {
        self.proof.provenance().into()
    }
    /// The original and backup are never conversion targets. Target refusal still
    /// returns backup provenance; indeterminate commit UUIDs are never replayed.
    pub async fn migrate(
        &self,
        source: &FolderRepository,
        original: &FolderReadSnapshot,
        target: &Path,
    ) -> MigrationReport {
        let report = backend::migrate_verified_v2(source, original, &self.proof, target).await;
        MigrationReport {
            backup: report.backup.into(),
            target: report.target.map(Publication::from),
        }
    }
}

#[cfg(test)]
mod tests;
