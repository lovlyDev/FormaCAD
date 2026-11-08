use super::{changed, ordinary};
use crate::core::Result;
use std::{
    ffi::c_void,
    fs::{File, OpenOptions},
    io::Write,
    os::windows::{
        fs::{FileExt, MetadataExt, OpenOptionsExt},
        io::AsRawHandle,
    },
    path::{Path, PathBuf},
};

#[repr(C)]
#[derive(Default, PartialEq, Eq)]
struct Information {
    attributes: u32,
    creation: [u32; 2],
    access: [u32; 2],
    write: [u32; 2],
    volume: u32,
    size: [u32; 2],
    links: u32,
    index: [u32; 2],
}
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetFileInformationByHandle(handle: *mut c_void, information: *mut Information) -> i32;
}
fn information(file: &File) -> Result<Information> {
    let mut information = Information::default();
    // SAFETY: live owned handle and documented initialized thirteen-DWORD layout.
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut information) } == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    if information.links != 1 || information.attributes & (0x400 | 0x10) != 0 {
        return Err(changed());
    }
    Ok(information)
}

pub(super) struct NewFile {
    file: File,
    path: PathBuf,
    volume: u32,
    index: [u32; 2],
    expected: Option<(String, usize)>,
}
impl NewFile {
    pub(super) fn create(directory: &Path, hash: &str) -> Result<Self> {
        let path = directory.join(hash);
        // CREATE_NEW never opens an existing CAS/database alias. Keep the owned
        // new handle denying write/delete sharing until SQL outcome resolution.
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .share_mode(1)
            .custom_flags(0x0020_0000 | 0x8000_0000)
            .open(&path)?;
        let info = information(&file)?;
        let result = Self {
            volume: info.volume,
            index: info.index,
            file,
            path,
            expected: None,
        };
        result.recheck()?;
        Ok(result)
    }
    pub(super) fn write(&mut self, bytes: &[u8]) -> Result<()> {
        self.file.write_all(bytes)?;
        self.file.sync_all()?;
        self.expected = Some((crate::artifacts::digest(bytes), bytes.len()));
        Ok(())
    }
    pub(super) fn recheck(&self) -> Result<()> {
        self.recheck_identity()?;
        if let Some((hash, size)) = &self.expected {
            let mut bytes = vec![0; *size];
            let mut offset = 0;
            while offset < bytes.len() {
                let count = self.file.seek_read(&mut bytes[offset..], offset as u64)?;
                if count == 0 {
                    return Err(changed());
                }
                offset += count;
            }
            if crate::artifacts::digest(&bytes) != *hash {
                return Err(changed());
            }
        }
        self.recheck_identity()
    }
    pub(super) fn recheck_identity(&self) -> Result<()> {
        let info = information(&self.file)?;
        let current = ordinary(&self.path)?;
        if info.volume != self.volume
            || info.index != self.index
            || current.file_attributes() & 0x400 != 0
            || current.len() != self.file.metadata()?.len()
        {
            return Err(changed());
        }
        if let Some((_, size)) = &self.expected {
            if current.len() != *size as u64 {
                return Err(changed());
            }
        }
        // The owned leaf and retained directory handles deny delete sharing;
        // checking this pathname does not open an existing raw CAS handle.
        Ok(())
    }
}

pub(super) fn sync_directory(_: &Path) -> Result<()> {
    // FILE_FLAG_WRITE_THROUGH plus FlushFileBuffers above uses Windows' reported
    // file/NTFS metadata flush semantics. Do not try flushing FolderDirectory's
    // read-attributes-only handle, or claim universal device/power-loss proof.
    Ok(())
}
