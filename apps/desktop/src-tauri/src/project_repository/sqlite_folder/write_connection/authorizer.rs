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
    context: *mut c_void,
    action: c_int,
    first: *const c_char,
    second: *const c_char,
    database: *const c_char,
    _: *const c_char,
) -> c_int {
    if !context.is_null()
        && unsafe { &*context.cast::<super::source::Source>() }
            .error()
            .is_some()
    {
        return abi::DENY;
    }
    // Private Source state is owned by the connection/VFS, never a UI token.
    let creating =
        !context.is_null() && unsafe { &*context.cast::<super::source::Source>() }.creating();
    if creating {
        let table = unsafe { text(first) }.unwrap_or("");
        let schema = unsafe { text(database) }.unwrap_or("");
        let tables = [
            "folder_format",
            "repository_state",
            "folder_commits",
            "projects",
            "project_history",
            "project_history_events",
            "sqlite_sequence",
        ];
        match action {
            // CREATE TABLE/INDEX from the frozen DDL only. format::initialize is
            // the sole SQL caller while this private authority is active.
            2 if schema == "main" && tables.contains(&table) => return abi::OK,
            1 if schema == "main"
                && tables.contains(&unsafe { text(second) }.unwrap_or(""))
                && (table == "project_history_events_project"
                    || table.starts_with("sqlite_autoindex_")) =>
            {
                return abi::OK
            }
            // SQLite's sqlite3RefillIndex authorizes REINDEX while compiling
            // CREATE INDEX as well. Only the frozen DDL's one explicit index
            // needs this action; normal writers must still reject REINDEX.
            27 if creation_index_refill(schema, table) => return abi::OK,
            18 | 23
                if schema == "main"
                    && (tables.contains(&table)
                        || table == "sqlite_master"
                        || table == "sqlite_schema") =>
            {
                return abi::OK
            }
            19 => {
                let value = unsafe { text(second) };
                if (table.eq_ignore_ascii_case("application_id") && value == Some("1178816561"))
                    || (table.eq_ignore_ascii_case("user_version") && value == Some("1"))
                {
                    return abi::OK;
                }
            }
            _ => {}
        }
    }
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

fn creation_index_refill(schema: &str, index: &str) -> bool {
    schema == "main" && index == "project_history_events_project"
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{ffi::CString, ptr};

    #[test]
    fn index_refill_is_limited_to_the_frozen_creation_index() {
        assert!(creation_index_refill(
            "main",
            "project_history_events_project"
        ));
        assert!(!creation_index_refill(
            "temp",
            "project_history_events_project"
        ));
        assert!(!creation_index_refill(
            "attached",
            "project_history_events_project"
        ));
        assert!(!creation_index_refill("main", "foreign_index"));
        assert!(!creation_index_refill(
            "main",
            "sqlite_autoindex_projects_1"
        ));
    }

    #[test]
    fn normal_authorizer_rejects_even_the_creation_index_refill() {
        let index = CString::new("project_history_events_project").unwrap();
        let schema = CString::new("main").unwrap();
        // A null context has no private creation authority.
        let result = unsafe {
            authorize(
                ptr::null_mut(),
                27,
                index.as_ptr(),
                ptr::null(),
                schema.as_ptr(),
                ptr::null(),
            )
        };
        assert_eq!(result, abi::DENY);
    }
}
