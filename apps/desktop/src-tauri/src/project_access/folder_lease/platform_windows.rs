//! Windows path pinning: every directory denies FILE_SHARE_DELETE until all
//! lease clones finish. Reparse points are opened themselves and rejected.
//! https://learn.microsoft.com/windows/win32/api/fileapi/nf-fileapi-createfilew
use super::{changed, unavailable};
use crate::core::Result;
use std::{
    ffi::c_void,
    fs::{File, OpenOptions},
    os::windows::{
        fs::{MetadataExt, OpenOptionsExt},
        io::AsRawHandle,
    },
    path::{Component, Path, PathBuf},
};

const FILE_READ_ATTRIBUTES: u32 = 0x80;
const FILE_SHARE_READ_WRITE: u32 = 0x3;
const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
const FILE_ID_INFO_CLASS: i32 = 18;

#[repr(C)]
#[derive(Default, PartialEq, Eq)]
struct Identity {
    volume: u64,
    identifier: [u8; 16],
}

#[repr(C)]
#[derive(Default)]
struct HandleInformation {
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
    fn GetFileInformationByHandleEx(
        handle: *mut c_void,
        class: i32,
        information: *mut c_void,
        size: u32,
    ) -> i32;
    fn GetFileInformationByHandle(handle: *mut c_void, information: *mut HandleInformation) -> i32;
}

fn identity(file: &File) -> Result<Identity> {
    let mut id = Identity::default();
    // SAFETY: live owned handle and initialized buffer with documented FILE_ID_INFO layout.
    let success = unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle(),
            FILE_ID_INFO_CLASS,
            (&mut id as *mut Identity).cast(),
            std::mem::size_of::<Identity>() as u32,
        )
    };
    if success == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(id)
}

fn regular(file: &File) -> Result<()> {
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(changed());
    }
    let mut info = HandleInformation::default();
    // SAFETY: BY_HANDLE_FILE_INFORMATION contains the documented thirteen DWORDs;
    // this initialized repr(C) buffer is valid for the live File handle.
    let success = unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) };
    if success == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    if info.links != 1 {
        return Err(changed());
    }
    Ok(())
}

fn directory(path: &Path) -> Result<File> {
    let file = OpenOptions::new()
        .access_mode(FILE_READ_ATTRIBUTES)
        .share_mode(FILE_SHARE_READ_WRITE)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_dir() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(changed());
    }
    Ok(file)
}

pub(super) struct Source {
    path: PathBuf,
    // Retaining all ancestors prevents path traversal through a renamed parent.
    directories: Vec<(PathBuf, File, Identity)>,
}

impl Source {
    pub(super) fn database_path(&self) -> PathBuf {
        self.path.join("project.sqlite")
    }

    pub(super) fn assets_directory(&self) -> Result<AssetDirectory> {
        self.recheck_directories()?;
        let path = self.path.join("assets");
        match std::fs::create_dir(&path) {
            Ok(()) => (),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => (),
            Err(error) => return Err(error.into()),
        }
        let file = directory(&path)?;
        let id = identity(&file)?;
        let retained = AssetDirectory { path, file, id };
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
        let path = std::path::absolute(root)?;
        if path.components().any(|part| {
            !matches!(
                part,
                Component::Prefix(_) | Component::RootDir | Component::Normal(_)
            )
        }) {
            return Err(unavailable());
        }
        let mut ancestors: Vec<_> = path.ancestors().map(Path::to_path_buf).collect();
        ancestors.reverse();
        let mut directories = Vec::new();
        for ancestor in ancestors {
            let file = directory(&ancestor)?;
            let id = identity(&file)?;
            directories.push((ancestor, file, id));
        }
        let mut source = Self { path, directories };
        source.recheck_directories()?;
        if require_empty {
            // The retained no-delete-share chain keeps this path on the first
            // opened root throughout enumeration and later initialization.
            let mut entries = std::fs::read_dir(&source.path)?;
            if let Some(entry) = entries.next() {
                entry?;
                return Err(unavailable());
            }
        }
        let inner_path = source.path.join(".forma");
        match std::fs::create_dir(&inner_path) {
            Ok(()) => (),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => (),
            Err(error) => return Err(error.into()),
        }
        let inner = directory(&inner_path)?;
        let id = identity(&inner)?;
        source.directories.push((inner_path, inner, id));
        source.recheck_directories()?;
        Ok(source)
    }

    fn recheck_directories(&self) -> Result<()> {
        for (path, file, id) in &self.directories {
            let current = directory(path).map_err(|_| changed())?;
            if identity(file)? != *id || identity(&current)? != *id {
                return Err(changed());
            }
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
            .share_mode(FILE_SHARE_READ_WRITE)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
            .open(self.path.join(".forma/writer.lock"))?;
        regular(&file)?;
        self.recheck_directories()?;
        Ok(file)
    }

    pub(super) fn recheck(&self, lock: &File) -> Result<()> {
        self.recheck_directories()?;
        regular(lock)?;
        // Attribute-only reads do not touch the exclusively locked byte range.
        let current = OpenOptions::new()
            .access_mode(FILE_READ_ATTRIBUTES)
            .share_mode(FILE_SHARE_READ_WRITE)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
            .open(self.path.join(".forma/writer.lock"))?;
        regular(&current)?;
        if identity(lock)? != identity(&current)? {
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
        let current = directory(&source.path.join("assets"))?;
        if identity(&self.file)? != self.id || identity(&current)? != self.id {
            return Err(changed());
        }
        source.recheck_directories()
    }
}
