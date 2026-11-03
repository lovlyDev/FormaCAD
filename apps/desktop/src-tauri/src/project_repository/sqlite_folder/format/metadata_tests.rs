//! Actual SQL authorizer proves metadata never reads event rows or exports history.
use super::*;
use std::ffi::{c_char, c_void, CStr};

type Authorizer = unsafe extern "C" fn(
    *mut c_void,
    i32,
    *const c_char,
    *const c_char,
    *const c_char,
    *const c_char,
) -> i32;
unsafe extern "C" {
    fn sqlite3_set_authorizer(
        database: *mut c_void,
        callback: Option<Authorizer>,
        context: *mut c_void,
    ) -> i32;
}
unsafe extern "C" fn deny_event_reads(
    _context: *mut c_void,
    action: i32,
    table: *const c_char,
    _column: *const c_char,
    _database: *const c_char,
    _trigger: *const c_char,
) -> i32 {
    // SQLITE_READ = 20, SQLITE_DENY = 1. SQLite owns the callback's string.
    if action == 20
        && !table.is_null()
        && unsafe { CStr::from_ptr(table) }.to_bytes() == b"project_history_events"
    {
        1
    } else {
        0
    }
}

#[tokio::test]
async fn metadata_succeeds_with_event_reads_denied_and_cannot_be_mistaken_for_full_archive() {
    let mut connection = SqliteConnection::connect("sqlite::memory:").await.unwrap();
    let project: Project = serde_json::from_value(serde_json::json!({
        "schemaVersion":1,"id":uuid::Uuid::new_v4().to_string(),"name":"Metadata SQL fixture",
        "units":"mm","agent":"codex","pinned":false,"createdAt":"2026-11-04T00:00:00Z",
        "updatedAt":"2026-11-04T00:00:00Z","currentRevision":null,"messages":[],
        "exports":[],"files":[],"revisions":[]
    }))
    .unwrap();
    assert!(matches!(
        initialize(
            &mut connection,
            &uuid::Uuid::new_v4().to_string(),
            &project,
            None
        )
        .await
        .unwrap(),
        CommitOutcome::Committed(_)
    ));
    {
        let mut handle = connection.lock_handle().await.unwrap();
        // SAFETY: SQLx parks the worker; the callback is static, without context.
        assert_eq!(
            unsafe {
                sqlite3_set_authorizer(
                    handle.as_raw_handle().as_ptr().cast(),
                    Some(deny_event_reads),
                    std::ptr::null_mut(),
                )
            },
            0
        );
    }
    let metadata = read_metadata(&mut connection).await.unwrap();
    assert_eq!(metadata.project().id, project.id);
    assert!(metadata.history().is_none());
    assert!(read(&mut connection).await.is_err());
    {
        let mut handle = connection.lock_handle().await.unwrap();
        // SAFETY: exclusive parked worker; remove our static callback.
        assert_eq!(
            unsafe {
                sqlite3_set_authorizer(
                    handle.as_raw_handle().as_ptr().cast(),
                    None,
                    std::ptr::null_mut(),
                )
            },
            0
        );
    }
    let full = read(&mut connection).await.unwrap();
    assert!(metadata.same_saved_state(&full));
    let history = full.into_history().unwrap();
    crate::project_history::validate_archive(&project, &history).unwrap();
    assert!(metadata.into_history().is_none());
}
