//! Linux x86-64/AArch64 flags from UAPI asm-generic/fcntl.h and x86 fcntl.h.
//! https://github.com/torvalds/linux/blob/master/include/uapi/asm-generic/fcntl.h
//! Each untrusted component is opened relative to an already pinned directory.
use super::{changed, unavailable};
use crate::core::Result;
use std::{
    fs::{File, Metadata, OpenOptions},
    os::{
        fd::AsRawFd,
        unix::fs::{MetadataExt, OpenOptionsExt},
    },
    path::{Component, Path, PathBuf},
};

const O_DIRECTORY: i32 = 0o200000;
const O_NOFOLLOW: i32 = 0o400000;

#[derive(Clone, Copy, PartialEq, Eq)]
struct Identity {
    device: u64,
    inode: u64,
}

fn identity(metadata: &Metadata) -> Identity {
    Identity {
        device: metadata.dev(),
        inode: metadata.ino(),
    }
}

fn alias(directory: &File) -> PathBuf {
    PathBuf::from(format!("/proc/self/fd/{}", directory.as_raw_fd()))
}

fn directory(path: &Path) -> Result<File> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(O_DIRECTORY | O_NOFOLLOW)
        .open(path)?;
    if !file.metadata()?.is_dir() {
        return Err(changed());
    }
    Ok(file)
}

fn capture_root(root: &Path) -> Result<(PathBuf, File)> {
    let absolute = std::path::absolute(root)?;
    let mut current = directory(Path::new("/"))?;
    for component in absolute.components() {
        match component {
            Component::RootDir => (),
            Component::Normal(name) => current = directory(&alias(&current).join(name))?,
            _ => return Err(unavailable()),
        }
    }
    Ok((absolute, current))
}

pub(super) struct Source {
    path: PathBuf,
    root: File,
    inner: File,
    root_id: Identity,
    inner_id: Identity,
}

impl Source {
    pub(super) fn database_path(&self) -> PathBuf {
        alias(&self.root).join("project.sqlite")
    }

    pub(super) fn assets_directory(&self) -> Result<AssetDirectory> {
        self.recheck_directories()?;
        let path = alias(&self.root).join("assets");
        match std::fs::create_dir(&path) {
            Ok(()) => (),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => (),
            Err(error) => return Err(error.into()),
        }
        let file = directory(&path)?;
        let id = identity(&file.metadata()?);
        let retained = AssetDirectory {
            path: alias(&file),
            file,
            id,
        };
        retained.recheck(self)?;
        Ok(retained)
    }

    pub(super) fn capture(root: &Path) -> Result<Self> {
        Self::capture_with_empty_requirement(root, false)
    }

    pub(super) fn initialize_empty(root: &Path) -> Result<Self> {
        Self::capture_with_empty_requirement(root, true)
    }

    fn capture_with_empty_requirement(root: &Path, require_empty: bool) -> Result<Self> {
        let (path, root) = capture_root(root)?;
        let root_id = identity(&root.metadata()?);
        if require_empty {
            // Enumerate the original pinned object, before creating any entry.
            // A nonempty folder is rejected without touching its contents.
            let mut entries = std::fs::read_dir(alias(&root))?;
            if let Some(entry) = entries.next() {
                entry?;
                return Err(unavailable());
            }
        }
        let (_, current) = capture_root(&path)?;
        if identity(&current.metadata()?) != root_id {
            return Err(changed());
        }
        let inner_path = alias(&root).join(".forma");
        match std::fs::create_dir(&inner_path) {
            Ok(()) => (),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => (),
            Err(error) => return Err(error.into()),
        }
        let inner = directory(&inner_path)?;
        let inner_id = identity(&inner.metadata()?);
        let source = Self {
            path,
            root,
            inner,
            root_id,
            inner_id,
        };
        source.recheck_directories()?;
        Ok(source)
    }

    fn recheck_directories(&self) -> Result<()> {
        let (_, root) = capture_root(&self.path).map_err(|_| changed())?;
        if identity(&root.metadata()?) != self.root_id
            || identity(&self.root.metadata()?) != self.root_id
            || identity(&self.inner.metadata()?) != self.inner_id
            || identity(&directory(&alias(&self.root).join(".forma"))?.metadata()?) != self.inner_id
        {
            return Err(changed());
        }
        Ok(())
    }

    pub(super) fn open_lock(&self) -> Result<File> {
        self.recheck_directories()?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .custom_flags(O_NOFOLLOW)
            .open(alias(&self.inner).join("writer.lock"))?;
        let metadata = file.metadata()?;
        if !metadata.is_file() || metadata.nlink() != 1 {
            return Err(changed());
        }
        self.recheck_directories()?;
        Ok(file)
    }

    pub(super) fn recheck(&self, lock: &File) -> Result<()> {
        self.recheck_directories()?;
        let actual = lock.metadata()?;
        let path = std::fs::symlink_metadata(alias(&self.inner).join("writer.lock"))?;
        if !actual.is_file()
            || actual.nlink() != 1
            || !path.is_file()
            || path.nlink() != 1
            || identity(&actual) != identity(&path)
        {
            return Err(changed());
        }
        self.recheck_directories()
    }
}

pub(super) struct AssetDirectory {
    path: PathBuf,
    file: File,
    id: Identity,
}

impl AssetDirectory {
    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    pub(super) fn recheck(&self, source: &Source) -> Result<()> {
        source.recheck_directories()?;
        let current = directory(&alias(&source.root).join("assets"))?;
        if identity(&self.file.metadata()?) != self.id || identity(&current.metadata()?) != self.id
        {
            return Err(changed());
        }
        source.recheck_directories()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retained_alias_cannot_be_redirected_by_a_parent_swap() {
        let temp = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let parent = temp.path().join("parent");
        let root = parent.join("project");
        std::fs::create_dir_all(&root).unwrap();
        let source = Source::capture(&root).unwrap();
        std::fs::rename(&parent, temp.path().join("original-parent")).unwrap();
        std::os::unix::fs::symlink(outside.path(), &parent).unwrap();
        assert!(source.open_lock().is_err());
        // The original directory alias still resolves the captured object,
        // never the replacement path. This is not a publication bypass.
        std::fs::write(alias(&source.inner).join("anchor-proof"), b"pinned").unwrap();
        assert_eq!(
            std::fs::read(
                temp.path()
                    .join("original-parent/project/.forma/anchor-proof")
            )
            .unwrap(),
            b"pinned"
        );
        assert_eq!(std::fs::read_dir(outside.path()).unwrap().count(), 0);
    }

    #[test]
    fn replaced_inner_directory_cannot_redirect_lock_creation() {
        let temp = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let source = Source::capture(temp.path()).unwrap();
        std::fs::rename(
            temp.path().join(".forma"),
            temp.path().join("original-forma"),
        )
        .unwrap();
        std::os::unix::fs::symlink(outside.path(), temp.path().join(".forma")).unwrap();
        assert!(source.open_lock().is_err());
        assert_eq!(std::fs::read_dir(outside.path()).unwrap().count(), 0);
        assert_eq!(
            std::fs::read_dir(temp.path().join("original-forma"))
                .unwrap()
                .count(),
            0
        );
    }
}
