//! Real SQL format tests, including a file-backed uncertain-COMMIT fixture.
//! No CAS or CAD claims.
use super::*;
use sqlx::{sqlite::SqliteConnectOptions, Connection};

async fn connection() -> SqliteConnection {
    SqliteConnection::connect_with(
        &SqliteConnectOptions::new()
            .in_memory(true)
            .foreign_keys(true),
    )
    .await
    .unwrap()
}
fn project() -> Project {
    let revision = uuid::Uuid::new_v4().to_string();
    let project: Project=serde_json::from_value(serde_json::json!({
        "schemaVersion":1,"id":uuid::Uuid::new_v4().to_string(),"name":"FCP1 SQL fixture",
        "units":"mm","agent":"codex","pinned":false,"createdAt":"2026-11-01T00:00:00Z",
        "updatedAt":"2026-11-01T00:00:00Z","currentRevision":revision,"messages":[],"exports":[],"files":[],
        "revisions":[{"id":revision,"parent":null,"createdAt":"2026-11-01T00:00:00Z","prompt":"synthetic model state",
            "parameters":{"kind":"box","width":40,"depth":20,"height":10,"thickness":2,"holeDiameter":0,"holes":0}}]
    })).unwrap();
    project.validate().unwrap();
    project
}
fn committed(outcome: CommitOutcome) -> Snapshot {
    match outcome {
        CommitOutcome::Committed(snapshot) => *snapshot,
        other => panic!("expected a committed SQL outcome, got {other:?}"),
    }
}
fn rejected(outcome: CommitOutcome) -> AppError {
    match outcome {
        CommitOutcome::RejectedBefore(error) => error,
        other => panic!("expected rejection before SQL COMMIT, got {other:?}"),
    }
}
async fn initialized(connection: &mut SqliteConnection, project: &Project) -> Snapshot {
    committed(
        initialize(connection, &uuid::Uuid::new_v4().to_string(), project, None)
            .await
            .unwrap(),
    )
}
async fn payload(connection: &mut SqliteConnection) -> String {
    sqlx::query_scalar("SELECT payload FROM projects")
        .fetch_one(connection)
        .await
        .unwrap()
}

#[tokio::test]
async fn rejected_sqlite_commit_preserves_known_identity_without_replaying_operation() {
    unsafe extern "C" {
        fn sqlite3_commit_hook(
            database: *mut std::ffi::c_void,
            callback: Option<unsafe extern "C" fn(*mut std::ffi::c_void) -> i32>,
            context: *mut std::ffi::c_void,
        ) -> *mut std::ffi::c_void;
    }
    unsafe extern "C" fn reject(_: *mut std::ffi::c_void) -> i32 {
        1
    }
    let temporary = tempfile::tempdir().unwrap();
    let options = SqliteConnectOptions::new()
        .filename(temporary.path().join("project.sqlite"))
        .create_if_missing(true)
        .foreign_keys(true);
    let mut connection = SqliteConnection::connect_with(&options).await.unwrap();
    let project = project();
    let before = initialized(&mut connection, &project).await;
    {
        let mut handle = connection.lock_handle().await.unwrap();
        // SAFETY: SQLx parks the worker. The callback is static and has no
        // context, and SQLite owns no Rust allocation through this hook.
        unsafe {
            sqlite3_commit_hook(
                handle.as_raw_handle().as_ptr().cast(),
                Some(reject),
                std::ptr::null_mut(),
            );
        }
    }
    let mut candidate = project.clone();
    candidate.name = "must not be replayed".into();
    let outcome = commit(&mut connection, &before, &candidate, None)
        .await
        .unwrap();
    let failed_id = match outcome {
        CommitOutcome::Indeterminate { commit_id, error } => {
            crate::security::valid_id(&commit_id).unwrap();
            assert_ne!(commit_id, before.expectation().commit_id());
            assert!(!error.to_string().is_empty());
            commit_id
        }
        other => panic!("COMMIT failure must retain its identity, got {other:?}"),
    };
    // A COMMIT error can leave SQLx's transaction bookkeeping out of sync with
    // SQLite's automatic rollback. Discard it; outcome resolution must use a
    // fresh connection, never attempt another transaction on the failed one.
    connection.close().await.unwrap();
    let mut connection =
        SqliteConnection::connect_with(&options.create_if_missing(false).read_only(true))
            .await
            .unwrap();
    let after = read(&mut connection).await.unwrap();
    assert!(before.same_saved_state(&after));
    assert_eq!(after.project().name, project.name);
    assert_eq!(
        resolve_commit(&mut connection, before.expectation(), &failed_id)
            .await
            .unwrap(),
        CommitPresence::Absent
    );
}

#[tokio::test]
async fn known_commit_resolution_distinguishes_later_head_and_another_repository() {
    let mut connection = connection().await;
    let project = project();
    let first = initialized(&mut connection, &project).await;
    let first_id = first.expectation().commit_id().to_owned();
    assert_eq!(
        resolve_commit(&mut connection, first.expectation(), &first_id)
            .await
            .unwrap(),
        CommitPresence::Present { is_head: true }
    );
    let mut renamed = project.clone();
    renamed.name = "later metadata publication".into();
    let second = committed(
        commit(&mut connection, &first, &renamed, None)
            .await
            .unwrap(),
    );
    assert_eq!(
        resolve_commit(&mut connection, first.expectation(), &first_id)
            .await
            .unwrap(),
        CommitPresence::Present { is_head: false }
    );
    assert_eq!(
        resolve_commit(
            &mut connection,
            first.expectation(),
            second.expectation().commit_id()
        )
        .await
        .unwrap(),
        CommitPresence::Present { is_head: true }
    );
    let mut other = self::connection().await;
    initialized(&mut other, &project).await;
    assert_eq!(
        resolve_commit(&mut other, first.expectation(), &first_id)
            .await
            .err()
            .unwrap()
            .to_string(),
        "PROJECT_SNAPSHOT_CHANGED"
    );
    assert!(second.same_saved_state(&read(&mut connection).await.unwrap()));
}

#[tokio::test]
async fn initializes_distinct_production_identity_and_reads_without_changing_source() {
    let mut connection = connection().await;
    let project = project();
    let before = serde_json::to_vec(&project).unwrap();
    let snapshot = initialized(&mut connection, &project).await;
    let expected = snapshot.expectation();
    assert_ne!(expected.repository_id(), expected.project_id());
    assert_eq!(expected.project_id(), project.id);
    assert_eq!(expected.revision(), project.current_revision.as_deref());
    assert_eq!(expected.sequence(), 1);
    crate::security::valid_id(expected.commit_id()).unwrap();
    let raw = payload(&mut connection).await;
    let again = read(&mut connection).await.unwrap();
    assert_eq!(again.expectation(), expected);
    assert_eq!(again.history_state(), snapshot.history_state());
    assert_eq!(again.history(), snapshot.history());
    let mut transaction = connection.begin_with("BEGIN IMMEDIATE").await.unwrap();
    let reserved = read_in_transaction(&mut transaction).await.unwrap();
    assert!(reserved.same_saved_state(&snapshot));
    transaction.rollback().await.unwrap();
    assert_eq!(payload(&mut connection).await, raw);
    assert_eq!(serde_json::to_vec(&project).unwrap(), before);
    assert_eq!(serde_json::to_vec(again.project()).unwrap(), before);
    let app: i64 = sqlx::query_scalar("PRAGMA application_id")
        .fetch_one(&mut connection)
        .await
        .unwrap();
    assert_eq!(app, APPLICATION_ID);
}

#[tokio::test]
async fn refuses_candidate_or_future_markers_without_installing_schema() {
    for (app, version) in [(0x4643_4231_i64, 1), (APPLICATION_ID, 2), (0, 1)] {
        let mut connection = connection().await;
        sqlx::query(&format!("PRAGMA application_id={app}"))
            .execute(&mut connection)
            .await
            .unwrap();
        sqlx::query(&format!("PRAGMA user_version={version}"))
            .execute(&mut connection)
            .await
            .unwrap();
        assert!(read(&mut connection).await.is_err());
        let error = rejected(
            initialize(
                &mut connection,
                &uuid::Uuid::new_v4().to_string(),
                &project(),
                None,
            )
            .await
            .unwrap(),
        );
        assert_eq!(error.to_string(), "PROJECT_FOLDER_FORMAT_UNSUPPORTED");
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_schema")
            .fetch_one(&mut connection)
            .await
            .unwrap();
        assert_eq!(count, 0);
        let unchanged: i64 = sqlx::query_scalar("PRAGMA application_id")
            .fetch_one(&mut connection)
            .await
            .unwrap();
        assert_eq!(unchanged, app);
    }
}

#[tokio::test]
async fn refuses_existing_unmarked_tables_and_same_repository_project_identity() {
    let mut connection = connection().await;
    sqlx::query("CREATE TABLE unrelated(value TEXT)")
        .execute(&mut connection)
        .await
        .unwrap();
    sqlx::query("INSERT INTO unrelated VALUES('keep')")
        .execute(&mut connection)
        .await
        .unwrap();
    rejected(
        initialize(
            &mut connection,
            &uuid::Uuid::new_v4().to_string(),
            &project(),
            None,
        )
        .await
        .unwrap(),
    );
    let value: String = sqlx::query_scalar("SELECT value FROM unrelated")
        .fetch_one(&mut connection)
        .await
        .unwrap();
    assert_eq!(value, "keep");
    let mut empty = self::connection().await;
    let project = project();
    rejected(
        initialize(&mut empty, &project.id, &project, None)
            .await
            .unwrap(),
    );
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_schema")
        .fetch_one(&mut empty)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn metadata_aba_changes_commit_identity_even_with_identical_project_and_revision() {
    let mut connection = connection().await;
    let original = project();
    let first = initialized(&mut connection, &original).await;
    let mut renamed = original.clone();
    renamed.name = "temporary metadata".into();
    let second = committed(
        commit(&mut connection, &first, &renamed, None)
            .await
            .unwrap(),
    );
    assert_eq!(second.history(), first.history());
    let third = committed(
        commit(&mut connection, &second, &original, None)
            .await
            .unwrap(),
    );
    assert_eq!(
        serde_json::to_vec(third.project()).unwrap(),
        serde_json::to_vec(first.project()).unwrap()
    );
    assert_eq!(
        third.expectation().revision(),
        first.expectation().revision()
    );
    assert_ne!(
        third.expectation().commit_id(),
        first.expectation().commit_id()
    );
    assert_eq!(third.expectation().sequence(), 3);
    assert_eq!(
        rejected(
            commit(&mut connection, &first, &renamed, None)
                .await
                .unwrap()
        )
        .to_string(),
        "PROJECT_SNAPSHOT_CHANGED"
    );
    assert_eq!(
        read(&mut connection).await.unwrap().expectation(),
        third.expectation()
    );
}

#[tokio::test]
async fn invalid_history_transition_rolls_back_payload_and_head_in_same_transaction() {
    let mut connection = connection().await;
    let original = project();
    let first = initialized(&mut connection, &original).await;
    let mut invalid = original.clone();
    invalid.name = "must roll back".into();
    invalid.revisions[0].parameters.width += 1.0;
    let before = payload(&mut connection).await;
    rejected(
        commit(&mut connection, &first, &invalid, None)
            .await
            .unwrap(),
    );
    assert_eq!(payload(&mut connection).await, before);
    assert_eq!(
        read(&mut connection).await.unwrap().expectation(),
        first.expectation()
    );
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM folder_commits")
        .fetch_one(&mut connection)
        .await
        .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn caller_uncommitted_changes_are_never_committed_by_a_reader() {
    let mut connection = connection().await;
    let original = project();
    let first = initialized(&mut connection, &original).await;
    let mut transaction = connection.begin().await.unwrap();
    sqlx::query("UPDATE projects SET name='uncommitted'")
        .execute(&mut *transaction)
        .await
        .unwrap();
    assert!(read::within(&mut transaction).await.is_err());
    transaction.rollback().await.unwrap();
    assert_eq!(
        read(&mut connection).await.unwrap().expectation(),
        first.expectation()
    );
    assert_eq!(
        read(&mut connection).await.unwrap().project().name,
        original.name
    );
}

#[tokio::test]
async fn rejects_tampered_payload_cursor_and_chain_seals_without_repair() {
    for statement in [
        "UPDATE projects SET payload=payload || ' '",
        "UPDATE project_history SET state_json=state_json || ' '",
        "UPDATE folder_commits SET seal_sha256=replace(seal_sha256,'a','b') || 'x'",
    ] {
        let mut connection = connection().await;
        initialized(&mut connection, &project()).await;
        sqlx::query(statement)
            .execute(&mut connection)
            .await
            .unwrap();
        let before = payload(&mut connection).await;
        assert!(read(&mut connection).await.is_err());
        assert_eq!(payload(&mut connection).await, before);
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM folder_commits")
            .fetch_one(&mut connection)
            .await
            .unwrap();
        assert_eq!(count, 1);
    }
}

#[tokio::test]
async fn executable_schema_is_compared_with_compiled_ddl_not_stored_fingerprint() {
    let mut connection = connection().await;
    let original = project();
    let first = initialized(&mut connection, &original).await;
    sqlx::raw_sql("CREATE TRIGGER extra BEFORE UPDATE ON projects BEGIN DELETE FROM project_history_events; END;").execute(&mut connection).await.unwrap();
    // An attacker may update stored metadata too; it never becomes the expected schema.
    sqlx::query("UPDATE folder_format SET schema_fingerprint=?")
        .bind("a".repeat(64))
        .execute(&mut connection)
        .await
        .unwrap();
    assert!(read(&mut connection).await.is_err());
    let mut changed = original.clone();
    changed.name = "must not execute trigger".into();
    assert_eq!(
        rejected(
            commit(&mut connection, &first, &changed, None)
                .await
                .unwrap()
        )
        .to_string(),
        "PROJECT_FOLDER_FORMAT_UNSUPPORTED"
    );
    assert_eq!(
        payload(&mut connection).await,
        serde_json::to_string(&original).unwrap()
    );
    let events: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM project_history_events")
        .fetch_one(&mut connection)
        .await
        .unwrap();
    assert_eq!(events, 1);
}

#[tokio::test]
async fn model_edit_undo_redo_and_imported_history_round_trip_in_folder_sql_only() {
    let mut connection = connection().await;
    let original = project();
    let first = initialized(&mut connection, &original).await;
    let mut edited = original.clone();
    let mut revision = original.revisions[0].clone();
    revision.id = uuid::Uuid::new_v4().to_string();
    revision.parent = original.current_revision.clone();
    revision.parameters.width += 1.0;
    edited.current_revision = Some(revision.id.clone());
    edited.revisions.push(revision);
    let mut current = committed(
        commit(&mut connection, &first, &edited, None)
            .await
            .unwrap(),
    );
    for direction in [
        crate::project_history::Direction::Undo,
        crate::project_history::Direction::Redo,
    ] {
        let (next, intent) = crate::project_history::prepare(
            current.project(),
            current.history_state(),
            direction,
            current.project().current_revision.as_deref(),
            &uuid::Uuid::new_v4().to_string(),
            "2026-11-01T01:00:00Z",
        )
        .unwrap();
        current = committed(
            commit(&mut connection, &current, &next, Some(&intent))
                .await
                .unwrap(),
        );
    }
    assert_eq!(current.expectation().sequence(), 4);
    crate::project_history::validate_archive(current.project(), current.history()).unwrap();
    let mut copy = current.project().clone();
    copy.id = uuid::Uuid::new_v4().to_string();
    let mut independent = self::connection().await;
    let copied = committed(
        initialize(
            &mut independent,
            &uuid::Uuid::new_v4().to_string(),
            &copy,
            Some(current.history()),
        )
        .await
        .unwrap(),
    );
    assert_eq!(copied.history(), current.history());
    assert_ne!(
        copied.expectation().repository_id(),
        current.expectation().repository_id()
    );
    assert_ne!(
        copied.expectation().project_id(),
        current.expectation().project_id()
    );
    assert_eq!(copied.project().revisions, current.project().revisions);
    assert_eq!(
        read(&mut connection).await.unwrap().expectation(),
        current.expectation()
    );
}

#[tokio::test]
async fn inline_or_invalid_initial_candidate_never_installs_partial_schema() {
    for data in [Some("data:application/octet-stream;base64,eA=="), None] {
        let mut connection = connection().await;
        let mut candidate = project();
        candidate.files.push(
            serde_json::from_value(
                serde_json::json!({"name":"source.step","size":1,"kind":"model","data":data}),
            )
            .unwrap(),
        );
        rejected(
            initialize(
                &mut connection,
                &uuid::Uuid::new_v4().to_string(),
                &candidate,
                None,
            )
            .await
            .unwrap(),
        );
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_schema")
            .fetch_one(&mut connection)
            .await
            .unwrap();
        assert_eq!(count, 0);
    }
}

#[tokio::test]
async fn same_commit_identity_cannot_hide_changed_saved_bytes_with_recomputed_checksums() {
    let mut connection = connection().await;
    let original = project();
    let first = initialized(&mut connection, &original).await;
    assert!(first.same_saved_state(&read(&mut connection).await.unwrap()));
    let mut modified = original.clone();
    modified.name = "manual same-commit metadata change".into();
    let payload = serde_json::to_string(&modified).unwrap();
    let cursor: String = sqlx::query_scalar("SELECT state_json FROM project_history")
        .fetch_one(&mut connection)
        .await
        .unwrap();
    let mut rewritten = CommitRecord {
        id: first.expectation().commit_id().to_owned(),
        parent: None,
        sequence: 1,
        revision: original.current_revision.clone(),
        project_hash: crate::artifacts::digest(payload.as_bytes()),
        cursor_hash: crate::artifacts::digest(cursor.as_bytes()),
        history_hash: crate::artifacts::digest(first.history()),
        seal: String::new(),
    };
    rewritten.seal = record_seal(
        first.expectation().repository_id(),
        &original.id,
        &rewritten,
    )
    .unwrap();
    sqlx::query("UPDATE projects SET name=?,payload=?")
        .bind(&modified.name)
        .bind(payload)
        .execute(&mut connection)
        .await
        .unwrap();
    sqlx::query("UPDATE folder_commits SET project_sha256=?,seal_sha256=? WHERE commit_id=?")
        .bind(&rewritten.project_hash)
        .bind(&rewritten.seal)
        .bind(&rewritten.id)
        .execute(&mut connection)
        .await
        .unwrap();
    let changed = read(&mut connection).await.unwrap();
    assert_eq!(changed.expectation(), first.expectation());
    assert_ne!(changed.project().name, first.project().name);
    assert!(!first.same_saved_state(&changed));
    assert!(read_compatible(&mut connection, &first).await.is_err());
    assert!(changed.same_saved_state(&read(&mut connection).await.unwrap()));
    let error = rejected(
        commit(&mut connection, &first, &original, None)
            .await
            .unwrap(),
    );
    assert_eq!(error.to_string(), "PROJECT_SNAPSHOT_CHANGED");
    assert!(changed.same_saved_state(&read(&mut connection).await.unwrap()));
}
