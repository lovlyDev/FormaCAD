//! Existing standalone files only. No application database, CAD or recovery
//! acceptance claim. Fixture setup deliberately uses the default SQLite writer.
#![cfg(any(target_os = "linux", windows))]
use super::*;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode},
    Executor,
};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

async fn binding(
    session: &super::super::session::FolderSession,
) -> super::super::session::ReadBinding {
    let snapshot = session.capture().await.unwrap();
    session.read_binding(&snapshot).await.unwrap()
}
fn published(lease: &FolderLease) -> std::sync::Arc<super::super::cas::PublishedCasBatch> {
    std::sync::Arc::new(
        super::super::cas::PreparedCasBatch::validate(&project(), &[])
            .unwrap()
            .publish_new(lease)
            .unwrap(),
    )
}
type Files = BTreeMap<PathBuf, Vec<u8>>;
fn files(root: &Path) -> Files {
    fn walk(root: &Path, at: &Path, result: &mut Files) {
        for entry in fs::read_dir(at).unwrap() {
            let path = entry.unwrap().path();
            let metadata = fs::symlink_metadata(&path).unwrap();
            assert!(!metadata.file_type().is_symlink());
            if metadata.is_dir() {
                walk(root, &path, result);
            } else {
                result.insert(
                    path.strip_prefix(root).unwrap().to_owned(),
                    fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut result = Files::new();
    walk(root, root, &mut result);
    result
}
fn project() -> crate::models::Project {
    serde_json::from_value(serde_json::json!({
        "schemaVersion":1,"id":uuid::Uuid::new_v4().to_string(),"name":"guarded writer fixture",
        "units":"mm","agent":"codex","pinned":false,"createdAt":"2026-11-01T00:00:00Z",
        "updatedAt":"2026-11-01T00:00:00Z","currentRevision":null,"messages":[],
        "exports":[],"files":[],"revisions":[]
    }))
    .unwrap()
}
async fn fixture(root: &Path) -> format::Snapshot {
    let lease = FolderLease::initialize_empty(root).unwrap();
    fs::create_dir(root.join("assets")).unwrap();
    let mut connection = SqliteConnection::connect_with(
        &SqliteConnectOptions::new()
            .filename(lease.database_path())
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(SqliteJournalMode::Delete),
    )
    .await
    .unwrap();
    let outcome = format::initialize(
        &mut connection,
        &uuid::Uuid::new_v4().to_string(),
        &project(),
        None,
    )
    .await
    .unwrap();
    connection.close().await.unwrap();
    match outcome {
        format::CommitOutcome::Committed(snapshot) => *snapshot,
        other => panic!("fixture initialization failed: {other:?}"),
    }
}
async fn lease_available(root: &Path) {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if FolderLease::acquire(root).is_ok() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("SQLite physical close must eventually release the folder lease");
}

#[tokio::test]
async fn unpromoted_connection_reads_but_cannot_write_recover_or_create_journals() {
    let temp = tempfile::tempdir().unwrap();
    let snapshot = fixture(temp.path()).await;
    let session = super::super::session::FolderSession::open(temp.path())
        .await
        .unwrap();
    let before = files(temp.path());
    let source = source::Source::capture(
        temp.path(),
        FolderLease::acquire(temp.path()).unwrap(),
        binding(&session).await,
    )
    .unwrap();
    let mut writer = WriteConnection::connect_unpromoted(source).await.unwrap();
    assert!(!writer.source.promoted());
    assert!(snapshot.same_saved_state(&format::read(writer.conn()).await.unwrap()));
    writer.conn().execute("BEGIN IMMEDIATE").await.unwrap();
    assert!(writer
        .conn()
        .execute("UPDATE projects SET name='unverified mutation'")
        .await
        .is_err());
    unsafe extern "C" {
        fn sqlite3_get_autocommit(database: *mut std::ffi::c_void) -> i32;
    }
    let active = {
        let mut handle = writer.conn().lock_handle().await.unwrap();
        // SAFETY: SQLx parks its worker and provides the live SQLite handle.
        // Inspect SQLite itself: raw BEGIN is not SQLx transaction-depth state.
        unsafe { sqlite3_get_autocommit(handle.as_raw_handle().as_ptr().cast()) == 0 }
    };
    if active {
        writer.conn().execute("ROLLBACK").await.unwrap();
    }
    assert!(writer.conn().execute("BEGIN EXCLUSIVE").await.is_err());
    assert!(writer
        .conn()
        .execute("PRAGMA journal_mode=WAL")
        .await
        .is_err());
    assert!(writer
        .conn()
        .execute("ATTACH ':memory:' AS extra")
        .await
        .is_err());
    writer.close().await.unwrap();
    assert_eq!(files(temp.path()), before);
}

#[tokio::test]
async fn verified_writer_commits_fcp1_and_keeps_lease_until_actual_close() {
    let temp = tempfile::tempdir().unwrap();
    let snapshot = fixture(temp.path()).await;
    let session = super::super::session::FolderSession::open(temp.path())
        .await
        .unwrap();
    let lease = FolderLease::acquire(temp.path()).unwrap();
    let mut writer = WriteConnection::open(temp.path(), lease, &snapshot, binding(&session).await)
        .await
        .unwrap();
    assert!(FolderLease::acquire(temp.path()).is_err());
    writer.recheck(&snapshot).await.unwrap();
    assert!(writer
        .conn()
        .execute("CREATE TABLE surprise(value)")
        .await
        .is_err());
    let mut next = snapshot.project().clone();
    next.name = "new persisted metadata".into();
    let after = match format::commit(writer.conn(), &snapshot, &next, None)
        .await
        .unwrap()
    {
        format::CommitOutcome::Committed(snapshot) => *snapshot,
        other => panic!("guarded commit failed: {other:?}"),
    };
    assert!(writer.recheck(&snapshot).await.is_err());
    writer.recheck(&after).await.unwrap();
    assert!(!temp.path().join("project.sqlite-journal").exists());
    assert_eq!(session.capture().await.unwrap().project().name, next.name);
    // SQLx Drop closes asynchronously. The VFS file retains its Source/lease
    // through base xClose, rather than releasing it with this Rust wrapper.
    drop(writer);
    lease_available(temp.path()).await;
    let mut reader = SqliteConnection::connect_with(
        &SqliteConnectOptions::new()
            .filename(temp.path().join("project.sqlite"))
            .read_only(true),
    )
    .await
    .unwrap();
    assert_eq!(
        format::read(&mut reader).await.unwrap().project().name,
        next.name
    );
    reader.close().await.unwrap();
}

#[tokio::test]
async fn wrong_captured_head_and_foreign_schema_are_rejected_without_writes() {
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    let expected = fixture(a.path()).await;
    let session = super::super::session::FolderSession::open(a.path())
        .await
        .unwrap();
    fixture(b.path()).await;
    let lease = FolderLease::acquire(b.path()).unwrap();
    let before = files(b.path());
    assert!(
        WriteConnection::open(b.path(), lease, &expected, binding(&session).await)
            .await
            .is_err()
    );
    lease_available(b.path()).await;
    assert_eq!(files(b.path()), before);
    let mut raw = SqliteConnection::connect_with(
        &SqliteConnectOptions::new().filename(b.path().join("project.sqlite")),
    )
    .await
    .unwrap();
    raw.execute("CREATE TABLE foreign_table(value)")
        .await
        .unwrap();
    raw.close().await.unwrap();
    let before = files(b.path());
    assert!(WriteConnection::open(
        b.path(),
        FolderLease::acquire(b.path()).unwrap(),
        &expected,
        binding(&session).await
    )
    .await
    .is_err());
    lease_available(b.path()).await;
    assert_eq!(files(b.path()), before);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancelling_connect_while_base_open_waits_does_not_release_its_lease() {
    let temp = tempfile::tempdir().unwrap();
    let expected = fixture(temp.path()).await;
    let session = super::super::session::FolderSession::open(temp.path())
        .await
        .unwrap();
    let source = source::Source::capture(
        temp.path(),
        FolderLease::acquire(temp.path()).unwrap(),
        binding(&session).await,
    )
    .unwrap();
    let (entered, waiting) = tokio::sync::oneshot::channel();
    let (release, ready) = std::sync::mpsc::sync_channel(1);
    *source.before_base_open.lock().unwrap() = Some(Box::new(move || {
        entered.send(()).map_err(|_| ())?;
        ready.recv_timeout(Duration::from_secs(5)).map_err(|_| ())
    }));
    let task = tokio::spawn(async move { WriteConnection::open_source(source, &expected).await });
    tokio::time::timeout(Duration::from_secs(5), waiting)
        .await
        .unwrap()
        .unwrap();
    task.abort();
    match task.await {
        Err(error) => assert!(error.is_cancelled()),
        Ok(_) => panic!("connect task must be cancelled"),
    }
    assert!(
        FolderLease::acquire(temp.path()).is_err(),
        "VFS xOpen still owns the source even after future cancellation"
    );
    release.send(()).unwrap();
    lease_available(temp.path()).await;
}

#[tokio::test]
async fn foreign_sidecars_are_refused_without_removal_or_recovery() {
    for suffix in ["-journal", "-wal", "-shm"] {
        let temp = tempfile::tempdir().unwrap();
        let expected = fixture(temp.path()).await;
        let session = super::super::session::FolderSession::open(temp.path())
            .await
            .unwrap();
        let original = binding(&session).await;
        fs::write(
            temp.path().join(format!("project.sqlite{suffix}")),
            b"foreign sidecar; do not repair",
        )
        .unwrap();
        let before = files(temp.path());
        assert!(WriteConnection::open(
            temp.path(),
            FolderLease::acquire(temp.path()).unwrap(),
            &expected,
            original
        )
        .await
        .is_err());
        lease_available(temp.path()).await;
        assert_eq!(files(temp.path()), before);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn real_hot_journal_created_after_preflight_is_never_recovered_by_connect() {
    let temp = tempfile::tempdir().unwrap();
    let expected = fixture(temp.path()).await;
    let session = super::super::session::FolderSession::open(temp.path())
        .await
        .unwrap();
    let source = source::Source::capture(
        temp.path(),
        FolderLease::acquire(temp.path()).unwrap(),
        binding(&session).await,
    )
    .unwrap();
    let root = temp.path().to_path_buf();
    let proof = std::sync::Arc::new(std::sync::Mutex::new(None));
    let recorded = std::sync::Arc::clone(&proof);
    *source.before_base_open.lock().unwrap() = Some(Box::new(move || {
        let output = std::process::Command::new(std::env::current_exe().map_err(|_| ())?)
            .args([
                "--exact",
                "project_repository::sqlite_folder::write_connection::tests::hot_journal_child",
                "--ignored",
                "--nocapture",
            ])
            .env("FORMA_FCP1_WRITER_CRASH_ROOT", &root)
            .output()
            .map_err(|_| ())?;
        if !output.status.success() {
            return Err(());
        }
        let journal = fs::read(root.join("project.sqlite-journal")).map_err(|_| ())?;
        if !journal.starts_with(&[0xd9, 0xd5, 0x05, 0xf9, 0x20, 0xa1, 0x63, 0xd7]) {
            return Err(());
        }
        *recorded.lock().map_err(|_| ())? = Some(files(&root));
        Ok(())
    }));
    let error = match WriteConnection::open_source(source, &expected).await {
        Err(error) => error,
        Ok(_) => panic!("hot source must refuse writer open"),
    };
    assert_eq!(error.to_string(), "PROJECT_SNAPSHOT_RECOVERY_REQUIRED");
    lease_available(temp.path()).await;
    let before = proof
        .lock()
        .unwrap()
        .take()
        .expect("real crash fixture must run after source preflight");
    assert_eq!(
        files(temp.path()),
        before,
        "writer connect must not recover, remove or truncate the hot journal"
    );
}

#[tokio::test]
#[ignore = "owned child-process crash fixture; parent invokes the exact test"]
async fn hot_journal_child() {
    let Some(root) = std::env::var_os("FORMA_FCP1_WRITER_CRASH_ROOT") else {
        return;
    };
    let root = PathBuf::from(root);
    assert!(root.is_absolute());
    let mut writer = SqliteConnection::connect_with(
        &SqliteConnectOptions::new()
            .filename(root.join("project.sqlite"))
            .create_if_missing(false)
            .journal_mode(SqliteJournalMode::Delete)
            .pragma("synchronous", "FULL"),
    )
    .await
    .unwrap();
    // Cache spilling forces durable rollback records before this process exits
    // without Rust/SQLite destructors. The parent confirms the real hot magic.
    writer.execute("PRAGMA cache_size=1; BEGIN IMMEDIATE; UPDATE projects SET payload=payload || CAST(zeroblob(2097152) AS TEXT)").await.unwrap();
    std::process::exit(0);
}

#[tokio::test]
async fn create_new_fcp1_closes_and_fresh_guarded_reader_validates() {
    let temp = tempfile::tempdir().unwrap();
    let lease = FolderLease::initialize_empty(temp.path()).unwrap();
    fs::create_dir(temp.path().join("assets")).unwrap();
    let project = project();
    let batch = published(&lease);
    let (mut writer, outcome) = WriteConnection::create(
        temp.path(),
        lease,
        &uuid::Uuid::new_v4().to_string(),
        &project,
        None,
        batch,
    )
    .await
    .unwrap();
    let snapshot = match outcome {
        format::CommitOutcome::Committed(snapshot) => snapshot,
        other => panic!("guarded creation failed: {other:?}"),
    };
    assert!(!writer.source.creating());
    assert!(writer
        .conn()
        .execute("CREATE TABLE unexpected(id INTEGER)")
        .await
        .is_err());
    assert!(writer
        .conn()
        .execute("PRAGMA user_version=2")
        .await
        .is_err());
    assert!(FolderLease::acquire(temp.path()).is_err());
    writer.close().await.unwrap();
    let mut reader = super::super::read_only::ReadOnlyDatabase::open(temp.path())
        .await
        .unwrap();
    let actual = format::read(reader.conn()).await.unwrap();
    assert!(snapshot.same_saved_state(&actual));
    reader.close().await.unwrap();
}

#[tokio::test]
async fn creation_refuses_existing_or_raced_leaf_without_overwriting() {
    for bytes in [
        b"".as_slice(),
        b"foreign database",
        b"SQLite format 3\0broken",
    ] {
        let temp = tempfile::tempdir().unwrap();
        let lease = FolderLease::initialize_empty(temp.path()).unwrap();
        fs::create_dir(temp.path().join("assets")).unwrap();
        fs::write(temp.path().join("project.sqlite"), bytes).unwrap();
        let before = files(temp.path());
        let batch = published(&lease);
        assert!(WriteConnection::create(
            temp.path(),
            lease,
            &uuid::Uuid::new_v4().to_string(),
            &project(),
            None,
            batch,
        )
        .await
        .is_err());
        assert_eq!(before, files(temp.path()));
    }
    let temp = tempfile::tempdir().unwrap();
    let lease = FolderLease::initialize_empty(temp.path()).unwrap();
    fs::create_dir(temp.path().join("assets")).unwrap();
    let batch = published(&lease);
    let source = source::Source::capture_new(temp.path(), lease, batch).unwrap();
    let target = temp.path().join("project.sqlite");
    *source.before_base_open.lock().unwrap() = Some(Box::new(move || {
        fs::write(target, b"raced foreign leaf").map_err(|_| ())
    }));
    assert!(WriteConnection::connect_unpromoted(source).await.is_err());
    assert_eq!(
        fs::read(temp.path().join("project.sqlite")).unwrap(),
        b"raced foreign leaf"
    );
}

#[tokio::test]
async fn interrupted_bootstrap_is_never_a_complete_project() {
    let temp = tempfile::tempdir().unwrap();
    let lease = FolderLease::initialize_empty(temp.path()).unwrap();
    fs::create_dir(temp.path().join("assets")).unwrap();
    let batch = published(&lease);
    let source = source::Source::capture_new(temp.path(), lease, batch).unwrap();
    let writer = WriteConnection::connect_unpromoted(source).await.unwrap();
    // Interruption before the one atomic initialization transaction: physical
    // zero-length DB exists but must not be accepted as a saved FCP1 project.
    writer.close().await.unwrap();
    let before = files(temp.path());
    assert!(super::super::read_only::ReadOnlyDatabase::open(temp.path())
        .await
        .is_err());
    assert_eq!(before, files(temp.path()));
    let lease = FolderLease::acquire(temp.path()).unwrap();
    let batch = published(&lease);
    assert!(WriteConnection::create(
        temp.path(),
        lease,
        &uuid::Uuid::new_v4().to_string(),
        &project(),
        None,
        batch,
    )
    .await
    .is_err());
    assert_eq!(before, files(temp.path()));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancelling_create_base_open_retains_initialization_lease() {
    let temp = tempfile::tempdir().unwrap();
    let lease = FolderLease::initialize_empty(temp.path()).unwrap();
    fs::create_dir(temp.path().join("assets")).unwrap();
    let batch = published(&lease);
    let source = source::Source::capture_new(temp.path(), lease, batch).unwrap();
    let (entered, waiting) = tokio::sync::oneshot::channel();
    let (release, ready) = std::sync::mpsc::sync_channel(1);
    *source.before_base_open.lock().unwrap() = Some(Box::new(move || {
        entered.send(()).map_err(|_| ())?;
        ready.recv_timeout(Duration::from_secs(5)).map_err(|_| ())
    }));
    let task = tokio::spawn(async move { WriteConnection::connect_unpromoted(source).await });
    tokio::time::timeout(Duration::from_secs(5), waiting)
        .await
        .unwrap()
        .unwrap();
    task.abort();
    assert!(task.await.is_err());
    assert!(FolderLease::acquire(temp.path()).is_err());
    release.send(()).unwrap();
    lease_available(temp.path()).await;
    assert!(super::super::read_only::ReadOnlyDatabase::open(temp.path())
        .await
        .is_err());
}

#[tokio::test]
async fn public_writer_open_and_create_futures_are_send() {
    fn assert_send<T: Send>(future: T) {
        drop(future);
    }
    let existing = tempfile::tempdir().unwrap();
    let snapshot = fixture(existing.path()).await;
    let session = super::super::session::FolderSession::open(existing.path())
        .await
        .unwrap();
    let lease = FolderLease::acquire(existing.path()).unwrap();
    assert_send(WriteConnection::open(
        existing.path(),
        lease,
        &snapshot,
        binding(&session).await,
    ));
    let fresh = tempfile::tempdir().unwrap();
    let lease = FolderLease::initialize_empty(fresh.path()).unwrap();
    fs::create_dir(fresh.path().join("assets")).unwrap();
    let project = project();
    let repository_id = uuid::Uuid::new_v4().to_string();
    let batch = published(&lease);
    assert_send(WriteConnection::create(
        fresh.path(),
        lease,
        &repository_id,
        &project,
        None,
        batch,
    ));
}

#[tokio::test]
async fn identical_copy_cannot_use_original_physical_binding() {
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    let expected = fixture(a.path()).await;
    let session = super::super::session::FolderSession::open(a.path())
        .await
        .unwrap();
    fs::create_dir(b.path().join("assets")).unwrap();
    fs::copy(
        a.path().join("project.sqlite"),
        b.path().join("project.sqlite"),
    )
    .unwrap();
    let lease = FolderLease::acquire(b.path()).unwrap();
    let before = files(b.path());
    assert!(
        WriteConnection::open(b.path(), lease, &expected, binding(&session).await)
            .await
            .is_err()
    );
    lease_available(b.path()).await;
    assert_eq!(before, files(b.path()));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn close_during_initial_writer_open_revokes_binding_and_preserves_source() {
    let temp = tempfile::tempdir().unwrap();
    let expected = fixture(temp.path()).await;
    let session = super::super::session::FolderSession::open(temp.path())
        .await
        .unwrap();
    let source = source::Source::capture(
        temp.path(),
        FolderLease::acquire(temp.path()).unwrap(),
        binding(&session).await,
    )
    .unwrap();
    let before = files(temp.path());
    let (entered, waiting) = tokio::sync::oneshot::channel();
    let (release, ready) = std::sync::mpsc::sync_channel(1);
    *source.before_base_open.lock().unwrap() = Some(Box::new(move || {
        entered.send(()).map_err(|_| ())?;
        ready.recv_timeout(Duration::from_secs(5)).map_err(|_| ())
    }));
    let task = tokio::spawn(async move { WriteConnection::open_source(source, &expected).await });
    waiting.await.unwrap();
    session.close();
    release.send(()).unwrap();
    assert!(task.await.unwrap().is_err());
    lease_available(temp.path()).await;
    assert_eq!(before, files(temp.path()));
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn identical_leaf_replacement_after_source_capture_is_not_adopted() {
    let temp = tempfile::tempdir().unwrap();
    let expected = fixture(temp.path()).await;
    let session = super::super::session::FolderSession::open(temp.path())
        .await
        .unwrap();
    let source = source::Source::capture(
        temp.path(),
        FolderLease::acquire(temp.path()).unwrap(),
        binding(&session).await,
    )
    .unwrap();
    let root = temp.path().to_owned();
    let original = fs::read(root.join("project.sqlite")).unwrap();
    fs::write(root.join("copy.sqlite"), &original).unwrap();
    *source.before_base_open.lock().unwrap() = Some(Box::new(move || {
        fs::rename(root.join("project.sqlite"), root.join("original.sqlite")).map_err(|_| ())?;
        fs::rename(root.join("copy.sqlite"), root.join("project.sqlite")).map_err(|_| ())
    }));
    assert!(WriteConnection::open_source(source, &expected)
        .await
        .is_err());
    lease_available(temp.path()).await;
    assert_eq!(
        original,
        fs::read(temp.path().join("project.sqlite")).unwrap()
    );
    assert_eq!(
        original,
        fs::read(temp.path().join("original.sqlite")).unwrap()
    );
    assert!(!temp.path().join("project.sqlite-journal").exists());
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn identical_leaf_aba_cannot_hide_the_actual_opened_copy() {
    let temp = tempfile::tempdir().unwrap();
    let expected = fixture(temp.path()).await;
    let session = super::super::session::FolderSession::open(temp.path())
        .await
        .unwrap();
    let source = source::Source::capture(
        temp.path(),
        FolderLease::acquire(temp.path()).unwrap(),
        binding(&session).await,
    )
    .unwrap();
    let root = temp.path().to_owned();
    let original = fs::read(root.join("project.sqlite")).unwrap();
    fs::write(root.join("copy.sqlite"), &original).unwrap();
    let before = root.clone();
    *source.before_base_open.lock().unwrap() = Some(Box::new(move || {
        fs::rename(
            before.join("project.sqlite"),
            before.join("original.sqlite"),
        )
        .map_err(|_| ())?;
        fs::rename(before.join("copy.sqlite"), before.join("project.sqlite")).map_err(|_| ())
    }));
    *source.after_base_open.lock().unwrap() = Some(Box::new(move || {
        fs::rename(root.join("project.sqlite"), root.join("copy.sqlite")).map_err(|_| ())?;
        fs::rename(root.join("original.sqlite"), root.join("project.sqlite")).map_err(|_| ())
    }));
    assert!(WriteConnection::open_source(source, &expected)
        .await
        .is_err());
    lease_available(temp.path()).await;
    assert_eq!(
        original,
        fs::read(temp.path().join("project.sqlite")).unwrap()
    );
    assert_eq!(original, fs::read(temp.path().join("copy.sqlite")).unwrap());
    assert!(!temp.path().join("project.sqlite-journal").exists());
}
