use super::FolderOperation;
use crate::{
    core::{AppError, Result},
    project_repository::location::LocationIdentity,
};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

pub(crate) struct BundleDestination {
    path: PathBuf,
    parent: LocationIdentity,
}

impl FolderOperation {
    pub(crate) fn bundle_destination(&self, path: PathBuf) -> Result<BundleDestination> {
        let parent = BundleDestination::parent(&path)?;
        let source_root = self.source.root().canonicalize()?;
        if parent.canonicalize()?.starts_with(source_root) {
            return Err(invalid());
        }
        let captured = BundleDestination {
            parent: LocationIdentity::capture(parent, Path::new(""))?,
            path,
        };
        captured.recheck()?;
        Ok(captured)
    }
}

impl BundleDestination {
    fn parent(path: &Path) -> Result<&Path> {
        if !path.is_absolute()
            || !path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("cadpack"))
        {
            return Err(invalid());
        }
        path.parent().ok_or_else(invalid)
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn recheck(&self) -> Result<()> {
        if !self
            .parent
            .recheck(Self::parent(&self.path)?, Path::new(""))?
        {
            return Err(invalid());
        }
        match fs::symlink_metadata(&self.path) {
            Ok(metadata) if metadata.is_file() => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
            Ok(_) => Err(invalid()),
        }
    }
}

fn invalid() -> AppError {
    AppError::Invalid("PROJECT_BUNDLE_DESTINATION_INVALID".into())
}
