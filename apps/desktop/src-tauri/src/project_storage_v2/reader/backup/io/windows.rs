//! Retained Windows directories deny delete sharing; outputs use CREATE_NEW.
use super::{changed, Result};
use std::{
    ffi::c_void,
    fs::{File, OpenOptions},
    io::Write,
    os::windows::{
        fs::{MetadataExt, OpenOptionsExt},
        io::AsRawHandle,
    },
    path::{Path, PathBuf},
};

#[repr(C)]
#[derive(Default, PartialEq, Eq)]
struct Identity {
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
fn identity(file: &File) -> Result<Identity> {
    let mut value = Identity::default();
    // SAFETY: documented FILE_ID_INFO buffer and a live owned handle.
    if unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle(),
            18,
            (&mut value as *mut Identity).cast(),
            std::mem::size_of::<Identity>() as u32,
        )
    } == 0
    {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(value)
}
fn directory(path: &Path) -> Result<File> {
    let file = OpenOptions::new()
        .access_mode(0x81)
        .share_mode(3)
        .custom_flags(0x0200_0000 | 0x0020_0000)
        .open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_dir() || metadata.file_attributes() & 0x400 != 0 {
        return Err(changed());
    }
    Ok(file)
}
pub(super) struct Directory {
    file: File,
    entry: PathBuf,
    id: Identity,
}
impl Directory {
    pub(super) fn open(path: &Path) -> Result<Self> {
        let file = directory(path)?;
        let id = identity(&file)?;
        Ok(Self {
            file,
            entry: path.into(),
            id,
        })
    }
    pub(super) fn create_directory(&self, name: &str) -> Result<Self> {
        self.recheck()?;
        let path = self.entry.join(name);
        std::fs::create_dir(&path)?;
        let child = Self::open(&path)?;
        self.recheck()?;
        child.recheck()?;
        Ok(child)
    }
    pub(super) fn recheck(&self) -> Result<()> {
        if identity(&self.file)? != self.id || identity(&directory(&self.entry)?)? != self.id {
            return Err(changed());
        }
        Ok(())
    }
    pub(super) fn write_new(&self, name: &str, bytes: &[u8]) -> Result<()> {
        self.recheck()?;
        let path = self.entry.join(name);
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .share_mode(1)
            .custom_flags(0x0020_0000 | 0x8000_0000)
            .open(&path)?;
        let id = identity(&file)?;
        let mut information = StandardInformation::default();
        // SAFETY: documented FILE_STANDARD_INFO and live owned handle. Reuse
        // the identical void-buffer GetFileInformationByHandleEx ABI signature.
        if unsafe {
            GetFileInformationByHandleEx(
                file.as_raw_handle(),
                1,
                (&mut information as *mut StandardInformation).cast(),
                std::mem::size_of::<StandardInformation>() as u32,
            )
        } == 0
        {
            return Err(std::io::Error::last_os_error().into());
        }
        if information.links != 1
            || information.directory != 0
            || file.metadata()?.file_attributes() & 0x400 != 0
        {
            return Err(changed());
        }
        file.write_all(bytes)?;
        file.sync_all()?;
        let named = std::fs::symlink_metadata(path)?;
        if identity(&file)? != id
            || !named.is_file()
            || named.file_attributes() & 0x400 != 0
            || named.len() != bytes.len() as u64
        {
            return Err(changed());
        }
        self.recheck()
    }
    pub(super) fn sync(&self) -> Result<()> {
        // CREATE_NEW outputs use WRITE_THROUGH + FlushFileBuffers. Directory
        // flush is not a portable Windows guarantee or hardware power-loss proof.
        self.recheck()
    }
}
