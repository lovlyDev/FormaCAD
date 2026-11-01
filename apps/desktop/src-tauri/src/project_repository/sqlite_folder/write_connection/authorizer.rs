//! Existing format writes only; no ATTACH, schema changes, writable-schema,
//! journal-mode switches or extension/filesystem functions.
use super::abi;
use std::ffi::{c_char, c_int, c_void, CStr};

unsafe fn text<'a>(value: *const c_char) -> Option<&'a str> {
    if value.is_null() {
        None
    } else {
        unsafe { CStr::from_ptr(value) }.to_str().ok()
    }
}

pub(super) unsafe extern "C" fn authorize(
    _: *mut c_void,
    action: c_int,
    first: *const c_char,
    second: *const c_char,
    database: *const c_char,
    _: *const c_char,
) -> c_int {
    match action {
        20 | 21 | 22 | 32 | 33 => abi::OK,
        // SQLITE_INSERT/UPDATE only for the immutable verified format tables.
        18 | 23 => {
            let table = unsafe { text(first) }.unwrap_or("");
            let schema = unsafe { text(database) }.unwrap_or("");
            if schema == "main"
                && [
                    "projects",
                    "project_history",
                    "project_history_events",
                    "folder_commits",
                    "repository_state",
                    "sqlite_sequence",
                ]
                .contains(&table)
            {
                abi::OK
            } else {
                abi::DENY
            }
        }
        31 => match unsafe { text(second) } {
            Some(name)
                if !["load_extension", "readfile", "writefile"]
                    .iter()
                    .any(|denied| name.eq_ignore_ascii_case(denied)) =>
            {
                abi::OK
            }
            _ => abi::DENY,
        },
        19 => {
            let name = unsafe { text(first) }.unwrap_or("");
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
                "synchronous",
            ]
            .iter()
            .any(|key| name.eq_ignore_ascii_case(key));
            if introspection || (query && second.is_null()) {
                abi::OK
            } else {
                abi::DENY
            }
        }
        _ => abi::DENY,
    }
}
