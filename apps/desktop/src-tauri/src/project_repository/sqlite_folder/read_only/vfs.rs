//! One process-lifetime non-default VFS, routing opaque names through weak sources.
use super::{abi, io, source};
use std::{
    ffi::{c_char, c_int, c_void, CStr},
    ptr,
    sync::OnceLock,
};

pub(super) const NAME: &str = "forma-folder-pure-read-346-v1";
const NAME_C: &[u8] = b"forma-folder-pure-read-346-v1\0";
const MAX_VIRTUAL: c_int = 128;
static REGISTRATION: OnceLock<bool> = OnceLock::new();

pub(super) fn register() -> bool {
    *REGISTRATION.get_or_init(|| unsafe { register_once() })
}
unsafe fn register_once() -> bool {
    // Pin the audited public ABI and base behavior. Upgrades require a new audit.
    if unsafe { abi::sqlite3_libversion_number() } != 3_046_000 {
        return false;
    }
    if !unsafe { abi::sqlite3_vfs_find(NAME_C.as_ptr().cast()) }.is_null() {
        return false;
    }
    #[cfg(windows)]
    let base = unsafe { abi::sqlite3_vfs_find(c"win32".as_ptr()) };
    #[cfg(target_os = "linux")]
    let base = {
        let mut base = unsafe { abi::sqlite3_vfs_find(c"unix-posix".as_ptr()) };
        if base.is_null() {
            // Pinned source selects posixIoFinder for Linux unix when locking
            // styles are disabled. Require working compile-option diagnostics.
            if unsafe { abi::sqlite3_compileoption_get(0) }.is_null()
                || unsafe { abi::sqlite3_compileoption_used(c"ENABLE_LOCKING_STYLE".as_ptr()) } != 0
            {
                return false;
            }
            base = unsafe { abi::sqlite3_vfs_find(c"unix".as_ptr()) };
        }
        base
    };
    if base.is_null() {
        return false;
    }
    let underlying = unsafe { &*base };
    if underlying.xOpen.is_none() || underlying.szOsFile <= 0 || underlying.szOsFile > 1024 * 1024 {
        return false;
    }
    let vfs = Box::new(abi::Vfs {
        iVersion: 1,
        szOsFile: std::mem::size_of::<io::File>() as c_int,
        mxPathname: MAX_VIRTUAL,
        pNext: ptr::null_mut(),
        zName: NAME_C.as_ptr().cast(),
        pAppData: base.cast(),
        xOpen: Some(open),
        xDelete: Some(delete),
        xAccess: Some(access),
        xFullPathname: Some(full_path),
        xDlOpen: Some(dl_open),
        xDlError: Some(dl_error),
        xDlSym: Some(dl_sym),
        xDlClose: Some(dl_close),
        xRandomness: Some(random),
        xSleep: Some(sleep),
        xCurrentTime: Some(time),
        xGetLastError: Some(last_error),
        xCurrentTimeInt64: None,
        xSetSystemCall: None,
        xGetSystemCall: None,
        xNextSystemCall: None,
    });
    let pointer = Box::into_raw(vfs);
    // Stable process lifetime, never default/unregistered; no source Arc lives here.
    if unsafe { abi::sqlite3_vfs_register(pointer, 0) } == abi::OK {
        true
    } else {
        unsafe {
            drop(Box::from_raw(pointer));
        }
        false
    }
}

unsafe fn name<'a>(pointer: *const c_char) -> Option<&'a str> {
    if pointer.is_null() {
        return None;
    }
    // SAFETY: SQLite supplies a live NUL-terminated filename for this invocation.
    let value = unsafe { CStr::from_ptr(pointer) }.to_str().ok()?;
    (value.len() < MAX_VIRTUAL as usize).then_some(value)
}
unsafe fn base(vfs: *mut abi::Vfs) -> *mut abi::Vfs {
    unsafe { (*vfs).pAppData.cast() }
}

unsafe extern "C" fn open(
    vfs: *mut abi::Vfs,
    path: *const c_char,
    file: *mut abi::File,
    flags: c_int,
    output: *mut c_int,
) -> c_int {
    if file.is_null() {
        return abi::CANTOPEN;
    }
    // On any failed open SQLite must see NULL methods and must not call xClose.
    unsafe {
        ptr::write(
            file.cast::<io::File>(),
            io::File {
                methods: ptr::null(),
                state: ptr::null_mut(),
            },
        );
    }
    let allowed = abi::OPEN_READONLY
        | abi::OPEN_MAIN_DB
        | abi::OPEN_URI
        | abi::OPEN_PRIVATECACHE
        | abi::OPEN_FULLMUTEX
        | abi::OPEN_NOMUTEX
        | abi::OPEN_EXRESCODE;
    if flags & (abi::OPEN_READONLY | abi::OPEN_MAIN_DB) != (abi::OPEN_READONLY | abi::OPEN_MAIN_DB)
        || flags & !allowed != 0
    {
        return abi::READONLY;
    }
    let Some(path) = (unsafe { name(path) }) else {
        return abi::CANTOPEN;
    };
    let Some((source, logical)) = source::lookup(path) else {
        return abi::CANTOPEN;
    };
    if logical != "project.sqlite" || source.check().is_err() {
        return abi::READONLY;
    }
    #[cfg(test)]
    {
        let hook = source
            .before_base_open
            .lock()
            .ok()
            .and_then(|mut hook| hook.take());
        if let Some(hook) = hook {
            if !matches!(
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(hook)),
                Ok(Ok(()))
            ) {
                source.fail(2);
                return abi::IOERR;
            }
        }
    }
    let alias = match source.binding.alias(logical) {
        Ok(alias) => alias,
        Err(_) => {
            source.fail(2);
            return abi::CANTOPEN;
        }
    };
    let parent = unsafe { base(vfs) };
    let state = match unsafe { open_state(parent, source, &alias) } {
        Ok(state) => state,
        Err(code) => return code,
    };
    if unsafe { state.source.attest(state.base) }.is_err() {
        state.source.fail(2);
        return abi::IOERR;
    }
    if !output.is_null() {
        unsafe {
            *output = abi::OPEN_READONLY;
        }
    }
    unsafe {
        (*file.cast::<io::File>()).state = Box::into_raw(state);
        (*file).pMethods = &io::METHODS;
    }
    abi::OK
}

/// Open binary CAS data via the base VFS's actual inode/close bookkeeping, not
/// through SQLite core parsing and not through an independent OS database FD.
pub(super) fn data_file(
    source: std::sync::Arc<source::Source>,
    alias: &CStr,
) -> crate::core::Result<Box<io::State>> {
    if !register() {
        return Err(super::unsupported());
    }
    // SAFETY: our registration is process-lifetime and never unregistered.
    let own = unsafe { abi::sqlite3_vfs_find(NAME_C.as_ptr().cast()) };
    if own.is_null() {
        return Err(super::unsupported());
    }
    unsafe { open_state(base(own), source, alias) }.map_err(|_| super::invalid())
}

unsafe fn open_state(
    parent: *mut abi::Vfs,
    source: std::sync::Arc<source::Source>,
    alias: &CStr,
) -> std::result::Result<Box<io::State>, c_int> {
    if alias.to_bytes().len() >= unsafe { (*parent).mxPathname } as usize {
        return Err(abi::CANTOPEN);
    }
    // sqlite3_create_filename preserves URI metadata/double NUL and lives through
    // base xClose, because the base VFS may retain this filename pointer.
    let filename = unsafe {
        abi::sqlite3_create_filename(alias.as_ptr(), c"".as_ptr(), c"".as_ptr(), 0, ptr::null())
    };
    if filename.is_null() {
        return Err(abi::NOMEM);
    }
    let allocation =
        unsafe { abi::sqlite3_malloc64((*parent).szOsFile as u64) }.cast::<abi::File>();
    if allocation.is_null() {
        unsafe {
            abi::sqlite3_free_filename(filename);
        }
        return Err(abi::NOMEM);
    }
    unsafe {
        ptr::write_bytes(allocation.cast::<u8>(), 0, (*parent).szOsFile as usize);
    }
    let state = Box::new(io::State {
        source,
        base: allocation,
        filename,
    });
    let mut actual_flags = 0;
    let Some(open) = (unsafe { (*parent).xOpen }) else {
        return Err(abi::CANTOPEN);
    };
    let rc = unsafe {
        open(
            parent,
            filename,
            allocation,
            abi::OPEN_READONLY | abi::OPEN_MAIN_DB,
            &mut actual_flags,
        )
    };
    if rc != abi::OK {
        return Err(rc);
    }
    if actual_flags & 1 == 0 || actual_flags & 2 != 0 || io::ensure_base_methods(&state).is_err() {
        state.source.fail(2);
        return Err(abi::READONLY);
    }
    Ok(state)
}

unsafe extern "C" fn delete(_: *mut abi::Vfs, _: *const c_char, _: c_int) -> c_int {
    abi::READONLY
}
unsafe extern "C" fn access(
    _: *mut abi::Vfs,
    path: *const c_char,
    flags: c_int,
    output: *mut c_int,
) -> c_int {
    if output.is_null() {
        return abi::IOERR;
    }
    unsafe {
        *output = 0;
    }
    let Some(path) = (unsafe { name(path) }) else {
        return abi::CANTOPEN;
    };
    let Some((source, logical)) = source::lookup(path) else {
        return abi::CANTOPEN;
    };
    // Examine sidecars in the pinned namespace. Never pretend an observed
    // journal/WAL/SHM is absent or permit SQLite to recover it implicitly.
    if source.check().is_err() {
        return abi::IOERR;
    }
    match std::fs::symlink_metadata(source.binding.path(logical)) {
        Ok(_) if logical != "project.sqlite" => {
            source.fail(1);
            abi::IOERR
        }
        Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => {
            unsafe {
                *output = i32::from(flags != 1);
            }
            abi::OK
        }
        Ok(_) => {
            source.fail(2);
            abi::IOERR
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => abi::OK,
        Err(_) => abi::IOERR,
    }
}
unsafe extern "C" fn full_path(
    _: *mut abi::Vfs,
    input: *const c_char,
    length: c_int,
    output: *mut c_char,
) -> c_int {
    let Some(value) = (unsafe { name(input) }) else {
        return abi::CANTOPEN;
    };
    if source::lookup(value).is_none() || length <= value.len() as c_int || output.is_null() {
        return abi::CANTOPEN;
    }
    unsafe {
        ptr::copy_nonoverlapping(value.as_ptr(), output.cast::<u8>(), value.len());
        *output.add(value.len()) = 0;
    }
    abi::OK
}
unsafe extern "C" fn dl_open(_: *mut abi::Vfs, _: *const c_char) -> *mut c_void {
    ptr::null_mut()
}
unsafe extern "C" fn dl_error(_: *mut abi::Vfs, length: c_int, output: *mut c_char) {
    if length > 0 && !output.is_null() {
        unsafe {
            *output = 0;
        }
    }
}
unsafe extern "C" fn dl_sym(
    _: *mut abi::Vfs,
    _: *mut c_void,
    _: *const c_char,
) -> Option<unsafe extern "C" fn()> {
    None
}
unsafe extern "C" fn dl_close(_: *mut abi::Vfs, _: *mut c_void) {}
unsafe extern "C" fn random(vfs: *mut abi::Vfs, length: c_int, output: *mut c_char) -> c_int {
    let base = unsafe { base(vfs) };
    match unsafe { (*base).xRandomness } {
        Some(random) => unsafe { random(base, length, output) },
        None => 0,
    }
}
unsafe extern "C" fn sleep(vfs: *mut abi::Vfs, duration: c_int) -> c_int {
    let base = unsafe { base(vfs) };
    match unsafe { (*base).xSleep } {
        Some(sleep) => unsafe { sleep(base, duration) },
        None => 0,
    }
}
unsafe extern "C" fn time(vfs: *mut abi::Vfs, output: *mut f64) -> c_int {
    let base = unsafe { base(vfs) };
    match unsafe { (*base).xCurrentTime } {
        Some(time) => unsafe { time(base, output) },
        None => abi::ERROR,
    }
}
unsafe extern "C" fn last_error(_: *mut abi::Vfs, length: c_int, output: *mut c_char) -> c_int {
    if length > 0 && !output.is_null() {
        unsafe {
            *output = 0;
        }
    }
    0
}
