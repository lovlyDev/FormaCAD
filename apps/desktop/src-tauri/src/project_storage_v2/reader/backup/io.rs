//! Pinned create-new output tree; no existing destination leaf is reused.
use crate::{
    core::{AppError, Result},
    project_access::folder_lease::FolderLease,
};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as platform;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows as platform;

pub(super) struct BackupTree {
    lease: FolderLease,
    root: platform::Directory,
    metadata: platform::Directory,
    generations: platform::Directory,
    generation: platform::Directory,
    assets: platform::Directory,
}

fn changed() -> AppError {
    AppError::Invalid("PROJECT_SNAPSHOT_CHANGED".into())
}

impl BackupTree {
    pub(super) fn create(lease: FolderLease, generation: &str) -> Result<Self> {
        crate::security::valid_id(generation)?;
        lease.recheck()?;
        let root_path = lease.database_path();
        let root = platform::Directory::open(root_path.parent().ok_or_else(changed)?)?;
        let metadata = root.create_directory("metadata")?;
        let generations = metadata.create_directory("generations")?;
        let generation = generations.create_directory(generation)?;
        let assets = root.create_directory("assets")?;
        let tree = Self {
            lease,
            root,
            metadata,
            generations,
            generation,
            assets,
        };
        tree.recheck()?;
        Ok(tree)
    }
    pub(super) fn recheck(&self) -> Result<()> {
        self.lease.recheck()?;
        for directory in [
            &self.root,
            &self.metadata,
            &self.generations,
            &self.generation,
            &self.assets,
        ] {
            directory.recheck()?;
        }
        self.lease.recheck()
    }
    pub(super) fn write_entry(&self, relative: &str, bytes: &[u8]) -> Result<()> {
        self.recheck()?;
        if relative == "project.json" || relative == "history.json" {
            self.generation.write_new(relative, bytes)?;
        } else if let Some(hash) = relative
            .strip_prefix("attachments/")
            .filter(|hash| crate::artifacts::valid_digest(hash))
        {
            self.assets.write_new(hash, bytes)?;
        } else {
            return Err(changed());
        }
        self.recheck()
    }
    pub(super) fn publish_manifest(&self, bytes: &[u8]) -> Result<()> {
        self.recheck()?;
        for directory in [
            &self.generation,
            &self.generations,
            &self.metadata,
            &self.assets,
        ] {
            directory.sync()?;
        }
        self.root.write_new("manifest.json", bytes)?;
        self.root.sync()?;
        self.recheck()
    }
}
