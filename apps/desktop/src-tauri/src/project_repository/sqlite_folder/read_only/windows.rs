//! Borrow the actual SQLite HANDLE; never close it or inspect private winFile.
use super::{abi, invalid};
use crate::core::Result;
use std::{
    ffi::{c_void, CString},
    fs::{File, OpenOptions},
    os::windows::{
        fs::{MetadataExt, OpenOptionsExt},
        io::AsRawHandle,
    },
    path::{Path, PathBuf},
};

#[repr(C)]
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct FileIdentity {
    volume: u64,
    identifier: [u8; 16],
}
pub(super) struct Binding {
    root: PathBuf,
    identity: FileIdentity,
    _directories: Vec<File>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct FileRecord {
    pub identity: FileIdentity,
    pub size: u64,
}
#[repr(C)]
#[derive(Default)]
struct StandardInfo {
    allocation: i64,
    end: i64,
    links: u32,
    delete_pending: u8,
    directory: u8,
    padding: [u8; 2],
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

fn handle_identity(handle: *mut c_void) -> Result<FileIdentity> {
    let mut identity = FileIdentity::default();
    // SAFETY: supplied owned/borrowed live handle and initialized FILE_ID_INFO.
    if unsafe {
        GetFileInformationByHandleEx(
            handle,
            18,
            (&mut identity as *mut FileIdentity).cast(),
            std::mem::size_of::<FileIdentity>() as u32,
        )
    } == 0
    {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(identity)
}

pub(super) fn path_identity(path: &Path) -> Result<FileIdentity> {
    let file = OpenOptions::new()
        .read(true)
        .access_mode(0x80)
        .share_mode(7)
        .custom_flags(0x0020_0000)
        .open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.file_attributes() & 0x400 != 0 {
        return Err(invalid());
    }
    Ok(file_record(file.as_raw_handle())?.identity)
}

fn directory(path: &Path, sharing: u32) -> Result<(File, FileIdentity)> {
    let file = OpenOptions::new()
        .read(true)
        .access_mode(0x80)
        .share_mode(sharing)
        .custom_flags(0x0200_0000 | 0x0020_0000)
        .open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_dir() || metadata.file_attributes() & 0x400 != 0 {
        return Err(invalid());
    }
    let identity = handle_identity(file.as_raw_handle())?;
    Ok((file, identity))
}
fn file_record(handle: *mut c_void) -> Result<FileRecord> {
    let identity = handle_identity(handle)?;
    let mut info = StandardInfo::default();
    // SAFETY: documented FILE_STANDARD_INFO layout and a live borrowed handle.
    if unsafe {
        GetFileInformationByHandleEx(
            handle,
            1,
            (&mut info as *mut StandardInfo).cast(),
            std::mem::size_of::<StandardInfo>() as u32,
        )
    } == 0
    {
        return Err(std::io::Error::last_os_error().into());
    }
    if info.links != 1 || info.directory != 0 || info.delete_pending != 0 || info.end < 0 {
        return Err(invalid());
    }
    Ok(FileRecord {
        identity,
        size: info.end as u64,
    })
}
unsafe fn sqlite_handle(inner: *mut abi::File) -> Result<*mut c_void> {
    let mut handle = std::ptr::null_mut::<c_void>();
    // SAFETY: public base method, initialized HANDLE output slot.
    let method = unsafe { (*(*inner).pMethods).xFileControl }.ok_or_else(invalid)?;
    if unsafe {
        method(
            inner,
            abi::WIN32_GET_HANDLE,
            (&mut handle as *mut *mut c_void).cast(),
        )
    } != abi::OK
        || handle.is_null()
    {
        return Err(invalid());
    }
    Ok(handle)
}

impl Binding {
    pub(super) fn capture(root: &Path) -> Result<Self> {
        let (file, identity) = directory(root, 7)?;
        Ok(Self {
            root: root.to_path_buf(),
            identity,
            _directories: vec![file],
        })
    }
    pub(super) fn path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }
    pub(super) fn alias(&self, name: &str) -> Result<CString> {
        CString::new(self.path(name).to_str().ok_or_else(invalid)?).map_err(|_| invalid())
    }
    pub(super) fn subdirectory(&self, name: &str) -> Result<Self> {
        let root = self.path(name);
        let mut ancestors: Vec<_> = root.ancestors().collect();
        ancestors.reverse();
        let mut handles = Vec::new();
        let mut identity = None;
        // Short-lived no-delete handles protect the directory chain during CAS
        // IO. Actual leaf identity is independently checked via SQLite HANDLE.
        for ancestor in ancestors {
            let (file, current) = directory(ancestor, 3)?;
            handles.push(file);
            identity = Some(current);
        }
        if !self.matches_directory(&self.root)? {
            return Err(invalid());
        }
        Ok(Self {
            root,
            identity: identity.ok_or_else(invalid)?,
            _directories: handles,
        })
    }
    pub(super) fn matches_directory(&self, path: &Path) -> Result<bool> {
        Ok(directory(path, 7)?.1 == self.identity)
    }
    pub(super) fn file_record(&self, name: &str) -> Result<FileRecord> {
        let file = OpenOptions::new()
            .read(true)
            .access_mode(0x80)
            .share_mode(7)
            .custom_flags(0x0020_0000)
            .open(self.path(name))?;
        let metadata = file.metadata()?;
        if !metadata.is_file() || metadata.file_attributes() & 0x400 != 0 {
            return Err(invalid());
        }
        file_record(file.as_raw_handle())
    }

    /// SAFETY: inner is a live base file, accessed only by xOpen or lock_handle.
    pub(super) unsafe fn actual_identity(
        &self,
        inner: *mut abi::File,
        name: &str,
    ) -> Result<FileIdentity> {
        let handle = unsafe { sqlite_handle(inner) }?;
        let actual = handle_identity(handle)?;
        if actual != path_identity(&self.path(name))? {
            return Err(invalid());
        }
        Ok(actual)
    }
    pub(super) unsafe fn actual_file_record(
        &self,
        inner: *mut abi::File,
        name: &str,
    ) -> Result<FileRecord> {
        let actual = file_record(unsafe { sqlite_handle(inner) }?)?;
        if actual != self.file_record(name)? {
            return Err(invalid());
        }
        Ok(actual)
    }
}
