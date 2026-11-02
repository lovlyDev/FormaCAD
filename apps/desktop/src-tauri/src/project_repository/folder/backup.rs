//! Nonserializable proof of a completely reread exact V2 generation backup.
use super::{FolderReadSnapshot, FolderRepository, Opening};
use crate::{
    core::{AppError, Result},
    models::{Project, ProjectFile},
};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Clone, Debug)]
pub(crate) struct BackupProvenance {
    pub(crate) root: PathBuf,
    pub(crate) manifest_sha256: String,
    pub(crate) generation: String,
    pub(crate) project_id: String,
}

pub(crate) struct VerifiedV2Backup {
    repository: FolderRepository,
    snapshot: FolderReadSnapshot,
    source_opening: Arc<Opening>,
    source_root: PathBuf,
    provenance: BackupProvenance,
}

fn changed() -> AppError {
    AppError::Invalid("PROJECT_SNAPSHOT_CHANGED".into())
}
fn canonical_directory(path: &Path) -> Result<PathBuf> {
    if !path.is_absolute() {
        return Err(changed());
    }
    for ancestor in path.ancestors() {
        let metadata = std::fs::symlink_metadata(ancestor)?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(changed());
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if metadata.file_attributes() & 0x400 != 0 {
                return Err(changed());
            }
        }
    }
    Ok(path.canonicalize()?)
}
fn disjoint(left: &Path, right: &Path) -> Result<()> {
    if left.starts_with(right) || right.starts_with(left) {
        return Err(changed());
    }
    Ok(())
}

impl FolderRepository {
    pub(crate) fn backup_committed(
        &self,
        original: &FolderReadSnapshot,
        destination: &Path,
    ) -> Result<VerifiedV2Backup> {
        self.recheck(original)?;
        let source_root = canonical_directory(&self.root)?;
        let destination = canonical_directory(destination)?;
        disjoint(&source_root, &destination)?;
        original
            .saved
            .backup_committed(&self.root, &destination, || self.recheck(original))?;
        let operation = (|| {
            self.recheck(original)?;
            let repository = FolderRepository::open(&destination)?;
            let snapshot = repository.capture()?;
            if snapshot.saved.manifest_sha256() != original.saved.manifest_sha256() {
                return Err(changed());
            }
            let provenance = BackupProvenance {
                root: destination.clone(),
                manifest_sha256: snapshot.saved.manifest_sha256().into(),
                generation: snapshot.saved.generation().into(),
                project_id: snapshot.project().id.clone(),
            };
            let verified = VerifiedV2Backup {
                repository,
                snapshot,
                source_opening: Arc::clone(&self.opening),
                source_root,
                provenance,
            };
            verified.recheck()?;
            verified.recheck_source(self, original)?;
            Ok(verified)
        })();
        operation.map_err(|error: AppError| {
            AppError::Invalid(format!(
            "PROJECT_BACKUP_INCOMPLETE; root={}; manifestSha256={}; generation={}; detail={error}",
            destination.display(), original.saved.manifest_sha256(), original.saved.generation()
        ))
        })
    }
}

impl VerifiedV2Backup {
    pub(crate) fn project(&self) -> &Project {
        self.snapshot.project()
    }
    pub(crate) fn read_history(&self) -> Result<Vec<u8>> {
        self.repository
            .read_history(&self.snapshot)?
            .ok_or_else(|| AppError::Invalid("HISTORY_ARCHIVE_REQUIRED".into()))
    }
    pub(crate) fn read_asset(&self, file: &ProjectFile) -> Result<Vec<u8>> {
        self.repository.read_asset(&self.snapshot, file)
    }
    pub(crate) fn recheck(&self) -> Result<()> {
        self.repository.recheck(&self.snapshot)?;
        self.snapshot.saved.verify_all(&self.repository.root)?;
        self.repository.recheck(&self.snapshot)
    }
    pub(crate) fn recheck_source(
        &self,
        source: &FolderRepository,
        original: &FolderReadSnapshot,
    ) -> Result<()> {
        if !Arc::ptr_eq(&self.source_opening, &source.opening)
            || !Arc::ptr_eq(&self.source_opening, &original.opening)
            || self.provenance.manifest_sha256 != original.saved.manifest_sha256()
            || canonical_directory(&source.root)? != self.source_root
        {
            return Err(changed());
        }
        source.recheck(original)?;
        original.saved.verify_all(&source.root)?;
        source.recheck(original)
    }
    pub(crate) fn provenance(&self) -> BackupProvenance {
        self.provenance.clone()
    }
    pub(crate) fn validate_target(&self, target: &Path) -> Result<()> {
        self.recheck()?;
        let target = canonical_directory(target)?;
        disjoint(&self.source_root, &target)?;
        disjoint(&self.provenance.root, &target)?;
        if let Some(entry) = std::fs::read_dir(target)?.next() {
            entry?;
            return Err(changed());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
