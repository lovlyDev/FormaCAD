//! SQLite file method policy. Version one intentionally has no SHM or mmap API.
use super::{abi, invalid, source::Source};
use std::{
    ffi::{c_int, c_void},
    ptr,
    sync::Arc,
};

#[repr(C)]
pub(super) struct File {
    pub methods: *const abi::IoMethods,
    pub state: *mut State,
}
pub(super) struct State {
    pub source: Arc<Source>,
    pub base: *mut abi::File,
    pub filename: *const std::ffi::c_char,
}

impl State {
    pub unsafe fn close(&mut self) -> c_int {
        if self.base.is_null() {
            return abi::OK;
        }
        // SAFETY: this allocation and successful/partial base open are owned here.
        let methods = unsafe { (*self.base).pMethods };
        let result = if !methods.is_null() {
            match unsafe { (*methods).xClose } {
                Some(close) => unsafe { close(self.base) },
                None => abi::IOERR,
            }
        } else {
            abi::OK
        };
        unsafe {
            abi::sqlite3_free(self.base.cast());
            abi::sqlite3_free_filename(self.filename);
        }
        self.base = ptr::null_mut();
        self.filename = ptr::null();
        result
    }
}
impl Drop for State {
    fn drop(&mut self) {
        /* SAFETY: unique owned allocations; close is idempotent. */
        unsafe {
            self.close();
        }
    }
}

unsafe fn state<'a>(file: *mut abi::File) -> &'a mut State {
    // SAFETY: only methods registered with our successfully initialized File call
    // this helper; SQLite/SQLx serialize calls for this file/connection.
    unsafe { &mut *(*(file.cast::<File>())).state }
}
unsafe fn methods<'a>(state: &State) -> &'a abi::IoMethods {
    unsafe { &*(*state.base).pMethods }
}

unsafe extern "C" fn close(file: *mut abi::File) -> c_int {
    // SAFETY: SQLite calls xClose once for a file with our methods installed.
    let wrapper = unsafe { &mut *file.cast::<File>() };
    let pointer = std::mem::replace(&mut wrapper.state, ptr::null_mut());
    wrapper.methods = ptr::null();
    if pointer.is_null() {
        return abi::OK;
    }
    let mut state = unsafe { Box::from_raw(pointer) };
    unsafe { state.close() }
}
unsafe extern "C" fn read(
    file: *mut abi::File,
    buffer: *mut c_void,
    amount: c_int,
    offset: i64,
) -> c_int {
    if amount < 0 || offset < 0 {
        return abi::IOERR;
    }
    let state = unsafe { state(file) };
    if state.source.error().is_some() {
        return abi::IOERR;
    }
    match unsafe { methods(state).xRead } {
        Some(read) => unsafe { read(state.base, buffer, amount, offset) },
        None => abi::IOERR,
    }
}
unsafe extern "C" fn write(_: *mut abi::File, _: *const c_void, _: c_int, _: i64) -> c_int {
    abi::READONLY
}
unsafe extern "C" fn truncate(_: *mut abi::File, _: i64) -> c_int {
    abi::READONLY
}
unsafe extern "C" fn sync(_: *mut abi::File, _: c_int) -> c_int {
    abi::READONLY
}
unsafe extern "C" fn size(file: *mut abi::File, output: *mut i64) -> c_int {
    let state = unsafe { state(file) };
    match unsafe { methods(state).xFileSize } {
        Some(size) => unsafe { size(state.base, output) },
        None => abi::IOERR,
    }
}
unsafe extern "C" fn lock(file: *mut abi::File, level: c_int) -> c_int {
    if level != 1 {
        return abi::READONLY;
    } // SHARED only, never recovery/write locks.
    let state = unsafe { state(file) };
    match unsafe { methods(state).xLock } {
        Some(lock) => unsafe { lock(state.base, level) },
        None => abi::IOERR,
    }
}
unsafe extern "C" fn unlock(file: *mut abi::File, level: c_int) -> c_int {
    if !(0..=1).contains(&level) {
        return abi::READONLY;
    }
    let state = unsafe { state(file) };
    match unsafe { methods(state).xUnlock } {
        Some(unlock) => unsafe { unlock(state.base, level) },
        None => abi::IOERR,
    }
}
unsafe extern "C" fn reserved(file: *mut abi::File, output: *mut c_int) -> c_int {
    let state = unsafe { state(file) };
    match unsafe { methods(state).xCheckReservedLock } {
        Some(check) => unsafe { check(state.base, output) },
        None => abi::IOERR,
    }
}
unsafe extern "C" fn control(
    file: *mut abi::File,
    operation: c_int,
    argument: *mut c_void,
) -> c_int {
    let state = unsafe { state(file) };
    if operation == abi::ATTEST {
        return match unsafe { state.source.attest(state.base) } {
            Ok(()) => abi::OK,
            Err(_) => {
                state.source.fail(2);
                abi::IOERR
            }
        };
    }
    match operation {
        // Read-only diagnostics/actual-handle identity, not a blanket delegation.
        1 | 4 | abi::HAS_MOVED | abi::WIN32_GET_HANDLE => {
            match unsafe { methods(state).xFileControl } {
                Some(control) => unsafe { control(state.base, operation, argument) },
                None => abi::NOTFOUND,
            }
        }
        // Known mutation/capability requests. SIZE_HINT can truncate/extend a file.
        3 | 5 | 6 | 9 | 10 | 11 | 13 | 16 | 18 | 21 | 22 | 23 | 24 | 26 | 31 | 32 | 33 | 34 => {
            abi::READONLY
        }
        _ => abi::NOTFOUND,
    }
}
unsafe extern "C" fn sector(file: *mut abi::File) -> c_int {
    let state = unsafe { state(file) };
    match unsafe { methods(state).xSectorSize } {
        Some(sector) => unsafe { sector(state.base) },
        None => 512,
    }
}
unsafe extern "C" fn characteristics(_: *mut abi::File) -> c_int {
    0
} // No IMMUTABLE/capability shortcuts.

pub(super) static METHODS: abi::IoMethods = abi::IoMethods {
    iVersion: 1,
    xClose: Some(close),
    xRead: Some(read),
    xWrite: Some(write),
    xTruncate: Some(truncate),
    xSync: Some(sync),
    xFileSize: Some(size),
    xLock: Some(lock),
    xUnlock: Some(unlock),
    xCheckReservedLock: Some(reserved),
    xFileControl: Some(control),
    xSectorSize: Some(sector),
    xDeviceCharacteristics: Some(characteristics),
    xShmMap: None,
    xShmLock: None,
    xShmBarrier: None,
    xShmUnmap: None,
    xFetch: None,
    xUnfetch: None,
};

pub(super) fn ensure_base_methods(state: &State) -> crate::core::Result<()> {
    // SAFETY: only called after base xOpen reports success and sets pMethods.
    let methods = unsafe { state.base.as_ref().and_then(|file| file.pMethods.as_ref()) }
        .ok_or_else(invalid)?;
    if methods.xClose.is_none()
        || methods.xRead.is_none()
        || methods.xFileSize.is_none()
        || methods.xLock.is_none()
        || methods.xUnlock.is_none()
        || methods.xCheckReservedLock.is_none()
        || methods.xFileControl.is_none()
    {
        return Err(invalid());
    }
    Ok(())
}
