//! Public SQLite 3.46 C ABI only; no private unixFile/winFile layout.
//! Audited against libsqlite3-sys 0.30.1's bundled sqlite3.h.
#![allow(non_snake_case)]
use std::ffi::{c_char, c_int, c_void};

pub(in crate::project_repository::sqlite_folder) const OK: c_int = 0;
pub(in crate::project_repository::sqlite_folder) const ERROR: c_int = 1;
pub(in crate::project_repository::sqlite_folder) const NOMEM: c_int = 7;
pub(in crate::project_repository::sqlite_folder) const READONLY: c_int = 8;
pub(in crate::project_repository::sqlite_folder) const IOERR: c_int = 10;
pub(in crate::project_repository::sqlite_folder) const NOTFOUND: c_int = 12;
pub(in crate::project_repository::sqlite_folder) const CANTOPEN: c_int = 14;
pub(in crate::project_repository::sqlite_folder) const DENY: c_int = 1;
pub(in crate::project_repository::sqlite_folder) const OPEN_READONLY: c_int = 1;
pub(in crate::project_repository::sqlite_folder) const OPEN_MAIN_DB: c_int = 0x100;
pub(in crate::project_repository::sqlite_folder) const OPEN_URI: c_int = 0x40;
pub(in crate::project_repository::sqlite_folder) const OPEN_PRIVATECACHE: c_int = 0x40000;
pub(in crate::project_repository::sqlite_folder) const OPEN_FULLMUTEX: c_int = 0x10000;
pub(in crate::project_repository::sqlite_folder) const OPEN_NOMUTEX: c_int = 0x8000;
pub(in crate::project_repository::sqlite_folder) const OPEN_EXRESCODE: c_int = 0x2000000;
pub(in crate::project_repository::sqlite_folder) const ATTEST: c_int = 0x464f524d;
pub(in crate::project_repository::sqlite_folder) const HAS_MOVED: c_int = 20;
pub(in crate::project_repository::sqlite_folder) const WIN32_GET_HANDLE: c_int = 29;

#[repr(C)]
pub(in crate::project_repository::sqlite_folder) struct File {
    pub pMethods: *const IoMethods,
}

#[repr(C)]
pub(in crate::project_repository::sqlite_folder) struct IoMethods {
    pub iVersion: c_int,
    pub xClose: Option<unsafe extern "C" fn(*mut File) -> c_int>,
    pub xRead: Option<unsafe extern "C" fn(*mut File, *mut c_void, c_int, i64) -> c_int>,
    pub xWrite: Option<unsafe extern "C" fn(*mut File, *const c_void, c_int, i64) -> c_int>,
    pub xTruncate: Option<unsafe extern "C" fn(*mut File, i64) -> c_int>,
    pub xSync: Option<unsafe extern "C" fn(*mut File, c_int) -> c_int>,
    pub xFileSize: Option<unsafe extern "C" fn(*mut File, *mut i64) -> c_int>,
    pub xLock: Option<unsafe extern "C" fn(*mut File, c_int) -> c_int>,
    pub xUnlock: Option<unsafe extern "C" fn(*mut File, c_int) -> c_int>,
    pub xCheckReservedLock: Option<unsafe extern "C" fn(*mut File, *mut c_int) -> c_int>,
    pub xFileControl: Option<unsafe extern "C" fn(*mut File, c_int, *mut c_void) -> c_int>,
    pub xSectorSize: Option<unsafe extern "C" fn(*mut File) -> c_int>,
    pub xDeviceCharacteristics: Option<unsafe extern "C" fn(*mut File) -> c_int>,
    pub xShmMap:
        Option<unsafe extern "C" fn(*mut File, c_int, c_int, c_int, *mut *mut c_void) -> c_int>,
    pub xShmLock: Option<unsafe extern "C" fn(*mut File, c_int, c_int, c_int) -> c_int>,
    pub xShmBarrier: Option<unsafe extern "C" fn(*mut File)>,
    pub xShmUnmap: Option<unsafe extern "C" fn(*mut File, c_int) -> c_int>,
    pub xFetch: Option<unsafe extern "C" fn(*mut File, i64, c_int, *mut *mut c_void) -> c_int>,
    pub xUnfetch: Option<unsafe extern "C" fn(*mut File, i64, *mut c_void) -> c_int>,
}

type Syscall = Option<unsafe extern "C" fn()>;
#[repr(C)]
pub(in crate::project_repository::sqlite_folder) struct Vfs {
    pub iVersion: c_int,
    pub szOsFile: c_int,
    pub mxPathname: c_int,
    pub pNext: *mut Vfs,
    pub zName: *const c_char,
    pub pAppData: *mut c_void,
    pub xOpen: Option<
        unsafe extern "C" fn(*mut Vfs, *const c_char, *mut File, c_int, *mut c_int) -> c_int,
    >,
    pub xDelete: Option<unsafe extern "C" fn(*mut Vfs, *const c_char, c_int) -> c_int>,
    pub xAccess: Option<unsafe extern "C" fn(*mut Vfs, *const c_char, c_int, *mut c_int) -> c_int>,
    pub xFullPathname:
        Option<unsafe extern "C" fn(*mut Vfs, *const c_char, c_int, *mut c_char) -> c_int>,
    pub xDlOpen: Option<unsafe extern "C" fn(*mut Vfs, *const c_char) -> *mut c_void>,
    pub xDlError: Option<unsafe extern "C" fn(*mut Vfs, c_int, *mut c_char)>,
    pub xDlSym: Option<unsafe extern "C" fn(*mut Vfs, *mut c_void, *const c_char) -> Syscall>,
    pub xDlClose: Option<unsafe extern "C" fn(*mut Vfs, *mut c_void)>,
    pub xRandomness: Option<unsafe extern "C" fn(*mut Vfs, c_int, *mut c_char) -> c_int>,
    pub xSleep: Option<unsafe extern "C" fn(*mut Vfs, c_int) -> c_int>,
    pub xCurrentTime: Option<unsafe extern "C" fn(*mut Vfs, *mut f64) -> c_int>,
    pub xGetLastError: Option<unsafe extern "C" fn(*mut Vfs, c_int, *mut c_char) -> c_int>,
    pub xCurrentTimeInt64: Option<unsafe extern "C" fn(*mut Vfs, *mut i64) -> c_int>,
    pub xSetSystemCall: Option<unsafe extern "C" fn(*mut Vfs, *const c_char, Syscall) -> c_int>,
    pub xGetSystemCall: Option<unsafe extern "C" fn(*mut Vfs, *const c_char) -> Syscall>,
    pub xNextSystemCall: Option<unsafe extern "C" fn(*mut Vfs, *const c_char) -> *const c_char>,
}

type Authorizer = Option<
    unsafe extern "C" fn(
        *mut c_void,
        c_int,
        *const c_char,
        *const c_char,
        *const c_char,
        *const c_char,
    ) -> c_int,
>;
unsafe extern "C" {
    pub fn sqlite3_libversion_number() -> c_int;
    #[cfg(target_os = "linux")]
    pub fn sqlite3_compileoption_get(index: c_int) -> *const c_char;
    #[cfg(target_os = "linux")]
    pub fn sqlite3_compileoption_used(option: *const c_char) -> c_int;
    pub fn sqlite3_vfs_find(name: *const c_char) -> *mut Vfs;
    pub fn sqlite3_vfs_register(vfs: *mut Vfs, make_default: c_int) -> c_int;
    pub fn sqlite3_malloc64(bytes: u64) -> *mut c_void;
    pub fn sqlite3_free(pointer: *mut c_void);
    pub fn sqlite3_create_filename(
        database: *const c_char,
        journal: *const c_char,
        wal: *const c_char,
        count: c_int,
        parameters: *const *const c_char,
    ) -> *const c_char;
    pub fn sqlite3_free_filename(filename: *const c_char);
    pub fn sqlite3_file_control(
        database: *mut c_void,
        schema: *const c_char,
        op: c_int,
        argument: *mut c_void,
    ) -> c_int;
    pub fn sqlite3_set_authorizer(
        database: *mut c_void,
        callback: Authorizer,
        context: *mut c_void,
    ) -> c_int;
}
