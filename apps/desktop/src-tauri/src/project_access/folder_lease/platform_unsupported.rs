//! No weak path-based writer fallback on unsupported platforms.
use super::unavailable;
use crate::core::Result;
use std::{
    fs::File,
    path::{Path, PathBuf},
};
pub(super) struct Source;
impl Source {
    pub(super) fn capture(_: &Path) -> Result<Self> {
        Err(unavailable())
    }
    pub(super) fn initialize_empty(_: &Path) -> Result<Self> {
        Err(unavailable())
    }
    pub(super) fn open_lock(&self) -> Result<File> {
        Err(unavailable())
    }
    pub(super) fn recheck(&self, _: &File) -> Result<()> {
        Err(unavailable())
    }
    pub(super) fn database_path(&self) -> PathBuf {
        PathBuf::new()
    }
    pub(super) fn assets_directory(&self) -> Result<AssetDirectory> {
        Err(unavailable())
    }
}
pub(super) struct AssetDirectory;
impl AssetDirectory {
    pub(super) fn path(&self) -> &Path {
        Path::new("")
    }
    pub(super) fn recheck(&self, _: &Source) -> Result<()> {
        Err(unavailable())
    }
}
