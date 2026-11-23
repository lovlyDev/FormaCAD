use super::{invalid, Result};
use std::{
    fs::{File, Metadata, OpenOptions},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
};

#[derive(PartialEq, Eq)]
pub(super) struct Identity {
    device: u64,
    inode: u64,
}
pub(super) fn check_metadata(metadata: &Metadata) -> Result<()> {
    if metadata.nlink() != 1 {
        return Err(invalid());
    }
    Ok(())
}
pub(super) fn configure(options: &mut OpenOptions) {
    options.custom_flags(0o400000);
}
pub(super) fn identity(file: &File) -> Result<Identity> {
    let metadata = file.metadata()?;
    check_metadata(&metadata)?;
    if !metadata.is_file() {
        return Err(invalid());
    }
    Ok(Identity {
        device: metadata.dev(),
        inode: metadata.ino(),
    })
}
pub(super) fn named_identity(_path: &Path, metadata: &Metadata) -> Result<Identity> {
    check_metadata(metadata)?;
    Ok(Identity {
        device: metadata.dev(),
        inode: metadata.ino(),
    })
}
