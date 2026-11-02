use super::{invalid, Result};
use std::{
    ffi::c_void,
    fs::{File, Metadata, OpenOptions},
    os::windows::{
        fs::{MetadataExt, OpenOptionsExt},
        io::AsRawHandle,
    },
    path::Path,
};

#[repr(C)]
#[derive(Default, PartialEq, Eq)]
pub(super) struct Identity {
    volume: u64,
    identifier: [u8; 16],
}
#[repr(C)]
#[derive(Default)]
struct StandardInformation {
    allocation_size: i64,
    end_of_file: i64,
    links: u32,
    delete_pending: u8,
    directory: u8,
}
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetFileInformationByHandleEx(
        handle: *mut c_void,
        class: i32,
        information: *mut c_void,
        size: u32,
    ) -> i32;
}

pub(super) fn check_metadata(metadata: &Metadata) -> Result<()> {
    if metadata.file_attributes() & 0x400 != 0 {
        return Err(invalid());
    }
    Ok(())
}
pub(super) fn configure(options: &mut OpenOptions) {
    options.share_mode(1).custom_flags(0x0020_0000);
}
pub(super) fn identity(file: &File) -> Result<Identity> {
    check_metadata(&file.metadata()?)?;
    let mut info = StandardInformation::default();
    let mut id = Identity::default();
    // SAFETY: documented FILE_STANDARD_INFO/FILE_ID_INFO and a live borrowed HANDLE.
    unsafe {
        if GetFileInformationByHandleEx(
            file.as_raw_handle(),
            1,
            (&mut info as *mut StandardInformation).cast(),
            std::mem::size_of::<StandardInformation>() as u32,
        ) == 0
            || GetFileInformationByHandleEx(
                file.as_raw_handle(),
                18,
                (&mut id as *mut Identity).cast(),
                std::mem::size_of::<Identity>() as u32,
            ) == 0
        {
            return Err(std::io::Error::last_os_error().into());
        }
    }
    if info.links != 1 || info.directory != 0 || info.delete_pending != 0 {
        return Err(invalid());
    }
    Ok(id)
}
pub(super) fn named_identity(path: &Path, _metadata: &Metadata) -> Result<Identity> {
    let mut options = OpenOptions::new();
    options.read(true);
    configure(&mut options);
    identity(&options.open(path)?)
}
