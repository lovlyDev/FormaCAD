//! Exact committed-generation backup. Unlisted tree entries are never copied.
use super::{bounded_read, changed, read_entry, reject_links, FolderSnapshot, MAX_MANIFEST};
use crate::{
    core::{AppError, Result},
    project_access::folder_lease::FolderLease,
};
use std::path::Path;

mod io;

impl FolderSnapshot {
    pub(crate) fn manifest_sha256(&self) -> &str {
        &self.seal
    }
    pub(crate) fn generation(&self) -> &str {
        &self.manifest.generation
    }

    /// Explicit full boundary check; ordinary per-asset reads stay bounded to
    /// their selected descriptor instead of repeatedly scanning every entry.
    pub(crate) fn verify_all(&self, root: &Path) -> Result<()> {
        self.recheck(root)?;
        for entry in &self.manifest.files {
            let bytes = read_entry(root, &self.manifest, entry)?;
            if entry.path == "history.json" {
                crate::project_history::validate_archive(&self.project, &bytes)?;
            }
        }
        self.recheck(root)
    }

    /// Caller retains original repository lifecycle/location authority through
    /// the callback. The supplied backup root must already exist and be empty.
    pub(crate) fn backup_committed(
        &self,
        source: &Path,
        destination: &Path,
        mut recheck_source: impl FnMut() -> Result<()>,
    ) -> Result<()> {
        recheck_source()?;
        self.recheck(source)?;
        // No silently synthesized history or destination writes for this case.
        self.read_history(source)?
            .ok_or_else(|| AppError::Invalid("HISTORY_ARCHIVE_REQUIRED".into()))?;
        self.verify_all(source)?;
        reject_links(destination)?;
        let manifest = bounded_read(source, Path::new("manifest.json"), MAX_MANIFEST)?;
        if crate::artifacts::digest(&manifest) != self.seal {
            return Err(changed());
        }
        recheck_source()?;
        let lease = FolderLease::initialize_empty(destination)?;
        let operation = (|| {
            recheck_source()?;
            let writer = io::BackupTree::create(lease, &self.manifest.generation)?;
            for entry in &self.manifest.files {
                recheck_source()?;
                let bytes = read_entry(source, &self.manifest, entry)?;
                recheck_source()?;
                writer.write_entry(&entry.path, &bytes)?;
            }
            recheck_source()?;
            self.recheck(source)?;
            writer.publish_manifest(&manifest)?;
            // File/directory authority survives the complete independent reread.
            self.verify_all(destination)?;
            recheck_source()?;
            writer.recheck()?;
            Ok(())
        })();
        operation.map_err(|error: AppError| {
            AppError::Invalid(format!(
            "PROJECT_BACKUP_INCOMPLETE; root={}; manifestSha256={}; generation={}; detail={error}",
            destination.display(), self.seal, self.manifest.generation
        ))
        })
    }
}

#[cfg(test)]
mod tests;
