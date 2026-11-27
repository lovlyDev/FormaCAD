//! SQL policy is defense in depth; VFS policy protects even connect-time PRAGMAs.
use super::abi;
use std::ffi::{c_char, c_int, c_void, CStr};

unsafe fn text<'a>(value: *const c_char) -> Option<&'a str> {
    if value.is_null() {
        None
    } else {
        // SAFETY: SQLite's authorizer arguments are live NUL-terminated strings
        // for this invocation only; callers never retain the result.
        unsafe { CStr::from_ptr(value) }.to_str().ok()
    }
}

pub(super) unsafe extern "C" fn authorize(
    _: *mut c_void,
    action: c_int,
    first: *const c_char,
    second: *const c_char,
    _: *const c_char,
    _: *const c_char,
) -> c_int {
    match action {
        // SQLITE_READ / SELECT / TRANSACTION / SAVEPOINT / RECURSIVE.
        20 | 21 | 22 | 32 | 33 => abi::OK,
        31 => {
            // SQLITE_FUNCTION: no extension/file system access.
            // SAFETY: arguments are supplied by SQLite during this call.
            match unsafe { text(second) } {
                Some(name)
                    if !["load_extension", "readfile", "writefile"]
                        .iter()
                        .any(|denied| name.eq_ignore_ascii_case(denied)) =>
                {
                    abi::OK
                }
                _ => abi::DENY,
            }
        }
        19 => {
            // SQLITE_PRAGMA: an explicit read-only list, never arbitrary PRAGMAs.
            // SAFETY: neither borrowed argument escapes this callback.
            let name = unsafe { text(first) }.unwrap_or("");
            let has_argument = !second.is_null();
            let introspection = [
                "table_info",
                "table_xinfo",
                "index_info",
                "index_xinfo",
                "index_list",
                "foreign_key_list",
            ]
            .iter()
            .any(|key| name.eq_ignore_ascii_case(key));
            let query = [
                "application_id",
                "user_version",
                "schema_version",
                "page_count",
                "page_size",
                "freelist_count",
                "journal_mode",
                "query_only",
                "temp_store",
                "foreign_keys",
                "encoding",
                "quick_check",
                "integrity_check",
                "database_list",
                "compile_options",
            ]
            .iter()
            .any(|key| name.eq_ignore_ascii_case(key));
            if introspection || (query && !has_argument) {
                abi::OK
            } else {
                abi::DENY
            }
        }
        _ => abi::DENY, // Includes ATTACH, DETACH, DDL and every data mutation.
    }
}
