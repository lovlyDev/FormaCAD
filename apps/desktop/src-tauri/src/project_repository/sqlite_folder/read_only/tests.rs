//! Actual SQLite files and a deterministic WAL switch after the last preflight.
//! These tests never replace native CAD/storage-format acceptance.
use super::*;

#[cfg(any(target_os = "linux", windows))]
mod supported {
    use super::*;
    use sqlx::{
        sqlite::{SqliteConnectOptions, SqliteJournalMode},
        Executor,
    };
    use std::{
        collections::BTreeMap,
        fs,
        path::{Path, PathBuf},
        sync::{Arc, Mutex},
        time::Duration,
    };

    #[derive(Debug, PartialEq, Eq)]
    enum Content {
        Directory,
        File(Vec<u8>),
    }
    type Files = BTreeMap<PathBuf, Content>;
    fn files(root: &Path) -> Files {
        fn walk(root: &Path, at: &Path, result: &mut Files) {
            for entry in fs::read_dir(at).unwrap() {
                let path = entry.unwrap().path();
                let metadata = fs::symlink_metadata(&path).unwrap();
                assert!(!metadata.file_type().is_symlink());
                let key = path.strip_prefix(root).unwrap().to_path_buf();
                if metadata.is_dir() {
                    result.insert(key, Content::Directory);
                    walk(root, &path, result);
                } else {
                    assert!(metadata.is_file());
                    result.insert(key, Content::File(fs::read(path).unwrap()));
                }
            }
        }
        let mut result = Files::new();
        walk(root, root, &mut result);
        result
    }
    async fn fixture(root: &Path) {
        let options = SqliteConnectOptions::new()
            .filename(root.join("project.sqlite"))
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Delete);
        let mut writer = SqliteConnection::connect_with(&options).await.unwrap();
        writer
            .execute("CREATE TABLE saved(value INTEGER NOT NULL); INSERT INTO saved VALUES(7)")
            .await
            .unwrap();
        writer.close().await.unwrap();
    }

    #[tokio::test]
    async fn read_selects_without_source_writes_and_refuses_sql_mutations() {
        let temp = tempfile::tempdir().unwrap();
        fixture(temp.path()).await;
        let before = files(temp.path());
        let mut database = ReadOnlyDatabase::open(temp.path()).await.unwrap();
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT value FROM saved")
                .fetch_one(database.conn())
                .await
                .unwrap(),
            7
        );
        for sql in [
            "INSERT INTO saved VALUES(8)",
            "PRAGMA journal_mode=WAL",
            "PRAGMA query_only=OFF",
            "PRAGMA user_version=9",
            "VACUUM",
            "ATTACH ':memory:' AS another",
        ] {
            assert!(
                database.conn().execute(sql).await.is_err(),
                "mutation must fail: {sql}"
            );
        }
        database.recheck().await.unwrap();
        assert_eq!(files(temp.path()), before);
        database.close().await.unwrap();
        assert_eq!(files(temp.path()), before);
    }

    #[tokio::test]
    async fn static_sidecars_are_not_recovered_or_removed() {
        for suffix in ["-journal", "-wal", "-shm"] {
            let temp = tempfile::tempdir().unwrap();
            fixture(temp.path()).await;
            fs::write(
                temp.path().join(format!("project.sqlite{suffix}")),
                b"untrusted sidecar",
            )
            .unwrap();
            let before = files(temp.path());
            let error = ReadOnlyDatabase::open(temp.path()).await.err().unwrap();
            assert_eq!(error.to_string(), "PROJECT_SNAPSHOT_RECOVERY_REQUIRED");
            assert_eq!(files(temp.path()), before);
        }
    }

    #[tokio::test]
    async fn clean_commits_keep_opening_identity_while_replacement_does_not() {
        let temp = tempfile::tempdir().unwrap();
        fixture(temp.path()).await;
        let mut database = ReadOnlyDatabase::open(temp.path()).await.unwrap();
        let initial_size = fs::metadata(temp.path().join("project.sqlite"))
            .unwrap()
            .len();
        let mut writer = SqliteConnection::connect_with(
            &SqliteConnectOptions::new().filename(temp.path().join("project.sqlite")),
        )
        .await
        .unwrap();
        writer.execute("UPDATE saved SET value=9; CREATE TABLE growth(data BLOB); INSERT INTO growth VALUES(zeroblob(65536))").await.unwrap();
        writer.close().await.unwrap();
        assert!(
            fs::metadata(temp.path().join("project.sqlite"))
                .unwrap()
                .len()
                > initial_size
        );
        database.recheck().await.unwrap();
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT value FROM saved")
                .fetch_one(database.conn())
                .await
                .unwrap(),
            9
        );
        database.recheck().await.unwrap();
        database.close().await.unwrap();
    }

    #[tokio::test]
    async fn cas_reads_verify_hash_size_and_leave_every_source_entry_unchanged() {
        let temp = tempfile::tempdir().unwrap();
        fixture(temp.path()).await;
        let assets = temp.path().join("assets");
        fs::create_dir(&assets).unwrap();
        let bytes = b"immutable fixture bytes";
        let hash = crate::artifacts::digest(bytes);
        fs::write(assets.join(&hash), bytes).unwrap();
        let before = files(temp.path());
        let mut database = ReadOnlyDatabase::open(temp.path()).await.unwrap();
        assert_eq!(
            database.read_blob(&hash, bytes.len() as u64).unwrap(),
            bytes
        );
        assert!(database.read_blob(&hash, bytes.len() as u64 + 1).is_err());
        assert!(database.read_blob("../project.sqlite", 10).is_err());
        database.recheck().await.unwrap();
        assert_eq!(files(temp.path()), before);
        fs::write(assets.join(&hash), b"different fixture bytes").unwrap();
        let corrupt = files(temp.path());
        assert!(database.read_blob(&hash, bytes.len() as u64).is_err());
        database.close().await.unwrap();
        assert_eq!(files(temp.path()), corrupt);
    }

    #[tokio::test]
    async fn malicious_cas_database_hardlink_cannot_release_writer_locks() {
        let temp = tempfile::tempdir().unwrap();
        fixture(temp.path()).await;
        let assets = temp.path().join("assets");
        fs::create_dir(&assets).unwrap();
        let database_bytes = fs::read(temp.path().join("project.sqlite")).unwrap();
        let hash = crate::artifacts::digest(&database_bytes);
        fs::hard_link(temp.path().join("project.sqlite"), assets.join(&hash)).unwrap();
        let mut database = ReadOnlyDatabase::open(temp.path()).await.unwrap();
        let before = files(temp.path());
        let mut writer = SqliteConnection::connect_with(
            &SqliteConnectOptions::new().filename(temp.path().join("project.sqlite")),
        )
        .await
        .unwrap();
        writer.execute("BEGIN EXCLUSIVE").await.unwrap();
        assert!(database
            .read_blob(&hash, database_bytes.len() as u64)
            .is_err());
        // An independent process observes the real OS lock, not SQLite's own
        // in-process bookkeeping. A raw DB-alias FD close would lose this lock
        // on Unix and let the child acquire EXCLUSIVE.
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--ignored",
                "--exact",
                "project_repository::sqlite_folder::read_only::tests::supported::child_lock_probe",
                "--nocapture",
            ])
            .env("FORMA_FOLDER_RO_LOCK_CHILD", temp.path())
            .output()
            .unwrap();
        assert!(
            child.status.success(),
            "{}",
            String::from_utf8_lossy(&child.stderr)
        );
        writer.execute("ROLLBACK").await.unwrap();
        writer.close().await.unwrap();
        database.recheck().await.unwrap();
        database.close().await.unwrap();
        assert_eq!(files(temp.path()), before);
    }

    #[tokio::test]
    #[ignore = "exact child process helper for the CAS-alias OS-lock regression"]
    async fn child_lock_probe() {
        let Some(root) = std::env::var_os("FORMA_FOLDER_RO_LOCK_CHILD") else {
            return;
        };
        let mut writer = SqliteConnection::connect_with(
            &SqliteConnectOptions::new()
                .filename(PathBuf::from(root).join("project.sqlite"))
                .busy_timeout(Duration::ZERO),
        )
        .await
        .unwrap();
        assert!(
            writer.execute("BEGIN EXCLUSIVE").await.is_err(),
            "parent's OS lock must remain held"
        );
        writer.close().await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn real_wal_switch_after_preflight_cannot_create_reader_shm_or_recover() {
        let temp = tempfile::tempdir().unwrap();
        fixture(temp.path()).await;
        let source = source::Source::capture(temp.path()).unwrap();
        let options = SqliteConnectOptions::new().filename(temp.path().join("project.sqlite"));
        let mut writer = SqliteConnection::connect_with(&options).await.unwrap();
        let (trigger, started) = tokio::sync::oneshot::channel::<()>();
        let (ready, receive) = std::sync::mpsc::sync_channel::<Files>(1);
        let (finish, finished) = tokio::sync::oneshot::channel::<()>();
        let root = temp.path().to_path_buf();
        let task = tokio::spawn(async move {
            if started.await.is_err() {
                return;
            }
            writer
                .execute("PRAGMA journal_mode=WAL; INSERT INTO saved VALUES(8)")
                .await
                .unwrap();
            ready.send(files(&root)).unwrap();
            let _ = finished.await;
            writer.close().await.unwrap();
        });
        let proof = Arc::new(Mutex::new(None));
        let recorded = Arc::clone(&proof);
        *source.before_base_open.lock().unwrap() = Some(Box::new(move || {
            trigger.send(()).map_err(|_| ())?;
            let before = receive
                .recv_timeout(Duration::from_secs(10))
                .map_err(|_| ())?;
            *recorded.lock().map_err(|_| ())? = Some(before);
            Ok(())
        }));
        // The source and final xOpen sidecar preflight were clean; the hook
        // switches real SQLite to WAL before underlying xOpen/header attestation.
        let error = ReadOnlyDatabase::open_source(source).await.err().unwrap();
        assert_eq!(error.to_string(), "PROJECT_SNAPSHOT_RECOVERY_REQUIRED");
        let before = proof
            .lock()
            .unwrap()
            .take()
            .expect("WAL switch handshake ran");
        assert!(before.contains_key(Path::new("project.sqlite-wal")));
        assert!(before.contains_key(Path::new("project.sqlite-shm")));
        assert_eq!(files(temp.path()), before);
        let _ = finish.send(());
        tokio::time::timeout(Duration::from_secs(10), task)
            .await
            .unwrap()
            .unwrap();
    }

    #[tokio::test]
    async fn file_controls_never_delegate_storage_mutations() {
        let temp = tempfile::tempdir().unwrap();
        fixture(temp.path()).await;
        let before = files(temp.path());
        let mut database = ReadOnlyDatabase::open(temp.path()).await.unwrap();
        {
            let mut handle = database.conn().lock_handle().await.unwrap();
            let mut size_hint = 64_i64 * 1024 * 1024;
            // SAFETY: SQLx exclusively parks the connection worker. SIZE_HINT
            // accepts an i64 pointer; our policy must refuse before delegation.
            let result = unsafe {
                abi::sqlite3_file_control(
                    handle.as_raw_handle().as_ptr().cast(),
                    c"main".as_ptr(),
                    5,
                    (&mut size_hint as *mut i64).cast(),
                )
            };
            assert_eq!(result, abi::READONLY);
            // Unknown controls are not delegated either.
            assert_eq!(
                unsafe {
                    abi::sqlite3_file_control(
                        handle.as_raw_handle().as_ptr().cast(),
                        c"main".as_ptr(),
                        7777,
                        std::ptr::null_mut(),
                    )
                },
                abi::NOTFOUND
            );
        }
        database.recheck().await.unwrap();
        database.close().await.unwrap();
        assert_eq!(files(temp.path()), before);
    }

    #[tokio::test]
    async fn default_writer_and_readonly_vfs_observe_each_others_shared_locks() {
        let temp = tempfile::tempdir().unwrap();
        fixture(temp.path()).await;
        let mut database = ReadOnlyDatabase::open(temp.path()).await.unwrap();
        database
            .conn()
            .execute("BEGIN; SELECT value FROM saved")
            .await
            .unwrap();
        let mut writer = SqliteConnection::connect_with(
            &SqliteConnectOptions::new()
                .filename(temp.path().join("project.sqlite"))
                .busy_timeout(Duration::ZERO),
        )
        .await
        .unwrap();
        assert!(writer.execute("BEGIN EXCLUSIVE").await.is_err());
        database.conn().execute("ROLLBACK").await.unwrap();
        writer.execute("BEGIN EXCLUSIVE; ROLLBACK").await.unwrap();
        database.recheck().await.unwrap();
        database.close().await.unwrap();
        writer.close().await.unwrap();
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn replacement_and_ancestor_swap_cannot_rebind_a_live_connection() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("source");
        fs::create_dir(&root).unwrap();
        fixture(&root).await;
        let mut database = ReadOnlyDatabase::open(&root).await.unwrap();
        fs::rename(root.join("project.sqlite"), root.join("old.sqlite")).unwrap();
        fixture(&root).await;
        let before = files(&root);
        assert_eq!(
            database.recheck().await.err().unwrap().to_string(),
            "PROJECT_SNAPSHOT_CHANGED"
        );
        database.close().await.unwrap();
        assert_eq!(files(&root), before);

        let source = source::Source::capture(&root).unwrap();
        let moved = temp.path().join("moved");
        fs::rename(&root, &moved).unwrap();
        std::os::unix::fs::symlink(&moved, &root).unwrap();
        let before = files(&moved);
        assert!(ReadOnlyDatabase::open_source(source).await.is_err());
        assert_eq!(files(&moved), before);
    }
}

#[cfg(not(any(target_os = "linux", windows)))]
#[tokio::test]
async fn unsupported_platform_refuses_before_any_sqlite_source_open() {
    let temp = tempfile::tempdir().unwrap();
    let error = ReadOnlyDatabase::open(temp.path()).await.err().unwrap();
    assert_eq!(error.to_string(), "FOLDER_SQLITE_READ_UNSUPPORTED");
    assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 0);
}
