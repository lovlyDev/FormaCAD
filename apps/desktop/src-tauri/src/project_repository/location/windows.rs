//! Windows 8+ file identity; unsupported filesystems fail closed.
//! https://learn.microsoft.com/windows/win32/api/winbase/ns-winbase-file_id_info
//! https://learn.microsoft.com/windows/win32/api/winbase/nf-winbase-getfileinformationbyhandleex
use std::{
    ffi::c_void,
    fs::{File, Metadata, OpenOptions},
    io,
    os::windows::{
        fs::{MetadataExt, OpenOptionsExt},
        io::AsRawHandle,
    },
    path::Path,
};

const FILE_READ_ATTRIBUTES: u32 = 0x80;
const FILE_SHARE_READ_WRITE_DELETE: u32 = 0x7;
const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
const FILE_ID_INFO_CLASS: i32 = 18;

#[repr(C)]
#[derive(Default, PartialEq, Eq)]
struct FileId {
    volume: u64,
    identifier: [u8; 16],
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

pub(super) struct Directory {
    // Share-delete permits rename tests while keeping the old file object alive.
    _handle: File,
    id: FileId,
}

impl Directory {
    pub(super) fn open(path: &Path) -> io::Result<Self> {
        let handle = OpenOptions::new()
            .read(true)
            .access_mode(FILE_READ_ATTRIBUTES)
            .share_mode(FILE_SHARE_READ_WRITE_DELETE)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
            .open(path)?;
        let metadata = handle.metadata()?;
        if !metadata.is_dir() || is_link(&metadata) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Directory identity changed",
            ));
        }
        let mut id = FileId::default();
        // SAFETY: the owned File supplies a live handle. FileId has FILE_ID_INFO's
        // documented C layout and a writable, correctly sized initialized buffer.
        let success = unsafe {
            GetFileInformationByHandleEx(
                handle.as_raw_handle(),
                FILE_ID_INFO_CLASS,
                (&mut id as *mut FileId).cast(),
                std::mem::size_of::<FileId>() as u32,
            )
        };
        if success == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self {
            _handle: handle,
            id,
        })
    }

    pub(super) fn same(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

pub(super) fn is_link(metadata: &Metadata) -> bool {
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}
