//! Optimistic path/OS identity checks. Retained read handles grant no writer lease.
use crate::core::{AppError, Result};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[cfg(unix)]
#[path = "location/unix.rs"]
mod platform;
#[cfg(windows)]
#[path = "location/windows.rs"]
mod platform;

pub(super) struct LocationIdentity {
    root: PathBuf,
    path: PathBuf,
    // Include existing intermediate directories when a managed leaf is absent.
    // A missing leaf becoming present changes this sequence and invalidates it.
    directories: Vec<(PathBuf, platform::Directory)>,
}

impl LocationIdentity {
    pub(super) fn capture(root: &Path, relative: &Path) -> Result<Self> {
        let captured = Self::capture_once(root, relative)?;
        // Check again after opening handles; path checks alone do not identify
        // what an open raced with. This remains optimistic, not an OS path lock.
        if !captured.matches(&Self::capture_once(root, relative)?) {
            return Err(super::legacy::changed());
        }
        Ok(captured)
    }

    pub(super) fn recheck(&self, root: &Path, relative: &Path) -> Result<bool> {
        match Self::capture_once(root, relative) {
            Ok(current) => Ok(self.matches(&current)),
            Err(AppError::Io(error)) if error.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error),
        }
    }

    fn capture_once(root: &Path, relative: &Path) -> Result<Self> {
        // The shared guard checks traversal before any handle is opened.
        let path = crate::security::guarded(root, relative)?;
        // guarded canonicalizes its root, so inspect the original chain too:
        // a symlink/junction root must not become trusted by canonicalization.
        reject_root_links(root)?;
        let canonical_root = root.canonicalize()?;
        let mut directories = vec![(
            canonical_root.clone(),
            platform::Directory::open(&canonical_root)?,
        )];
        let mut current = canonical_root.clone();
        let mut missing = false;
        for component in relative.components() {
            current.push(component);
            if missing {
                continue;
            }
            match fs::symlink_metadata(&current) {
                Ok(metadata) => {
                    reject_link_or_file(&metadata)?;
                    directories.push((current.clone(), platform::Directory::open(&current)?));
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => missing = true,
                Err(error) => return Err(error.into()),
            }
        }
        Ok(Self {
            root: canonical_root,
            path,
            directories,
        })
    }

    fn matches(&self, other: &Self) -> bool {
        self.root == other.root
            && self.path == other.path
            && self.directories.len() == other.directories.len()
            && self.directories.iter().zip(&other.directories).all(
                |((path, directory), (other_path, other_directory))| {
                    path == other_path && directory.same(other_directory)
                },
            )
    }
}

fn reject_root_links(root: &Path) -> Result<()> {
    let absolute = std::path::absolute(root)?;
    for ancestor in absolute.ancestors() {
        reject_link_or_file(&fs::symlink_metadata(ancestor)?)?;
    }
    Ok(())
}

fn reject_link_or_file(metadata: &fs::Metadata) -> Result<()> {
    if platform::is_link(metadata) || !metadata.is_dir() {
        return Err(super::legacy::changed());
    }
    Ok(())
}

#[cfg(test)]
#[path = "location/tests.rs"]
mod tests;
