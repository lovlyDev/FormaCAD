use super::{fixture, tree, A, B};
use crate::project_repository::{
    folder::FolderRepository,
    sqlite_folder::{
        crash_probe::{Point, POINT, READY},
        format::CommitOutcome,
        migration::{migrate_inner, migrate_verified_v2},
        session::FolderSession,
    },
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Child, Command},
    time::Duration,
};

struct MigratingChild(Child);

impl Drop for MigratingChild {
    fn drop(&mut self) {
        if matches!(self.0.try_wait(), Ok(None)) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

async fn park_child(
    source: &Path,
    backup: &Path,
    target: &Path,
    barrier: &Path,
    point: Point,
) -> MigratingChild {
    let (_, module) = module_path!().split_once("::").unwrap();
    let test = format!("{module}::run_until_migration_boundary");
    let mut child = MigratingChild(
        Command::new(std::env::current_exe().unwrap())
            .args(["--ignored", "--exact", &test, "--nocapture"])
            .env("FORMA_MIGRATION_SOURCE", source)
            .env("FORMA_MIGRATION_BACKUP", backup)
            .env("FORMA_MIGRATION_TARGET", target)
            .env(POINT, point.name())
            .env(READY, barrier)
            .spawn()
            .unwrap(),
    );
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            assert!(child.0.try_wait().unwrap().is_none(), "{point:?}");
            if fs::read_to_string(barrier).is_ok_and(|ready| ready == point.name()) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("migration did not reach {point:?}"));
    child
}

#[tokio::test]
#[ignore = "runs only as a child of the migration crash test"]
async fn run_until_migration_boundary() {
    let path = |name| PathBuf::from(std::env::var_os(name).expect("parent supplies path"));
    let source = FolderRepository::open(&path("FORMA_MIGRATION_SOURCE")).unwrap();
    let original = source.capture().unwrap();
    let backup = source
        .backup_committed(&original, &path("FORMA_MIGRATION_BACKUP"))
        .unwrap();
    let _ = migrate_inner(
        &source,
        &original,
        &backup,
        &path("FORMA_MIGRATION_TARGET"),
        Some(Box::new(|| Point::AfterSqlCommit.wait_for_kill())),
    )
    .await;
    panic!("migration must park at the requested process barrier");
}

#[tokio::test]
async fn interrupted_migration_preserves_verified_backup_and_can_be_reproduced() {
    for point in [Point::BeforeSqlCommit, Point::AfterSqlCommit] {
        let source_root = tempfile::tempdir().unwrap();
        let backup_root = tempfile::tempdir().unwrap();
        let target_root = tempfile::tempdir().unwrap();
        let control = tempfile::tempdir().unwrap();
        let source = fixture(source_root.path());
        let original = source.capture().unwrap();
        let expected_history = source.read_history(&original).unwrap().unwrap();
        let source_before = tree(source_root.path());
        let mut child = park_child(
            source_root.path(),
            backup_root.path(),
            target_root.path(),
            &control.path().join("parked"),
            point,
        )
        .await;
        let backup_before = tree(backup_root.path());
        assert!(!backup_before.is_empty());
        child.0.kill().unwrap();
        assert!(!child.0.wait().unwrap().success());

        assert_eq!(tree(source_root.path()), source_before);
        assert_eq!(tree(backup_root.path()), backup_before);
        let retained = FolderRepository::open(backup_root.path()).unwrap();
        let retained_view = retained.capture().unwrap();
        assert_eq!(retained_view.project().id, original.project().id);
        assert_eq!(
            retained.read_history(&retained_view).unwrap().unwrap(),
            expected_history
        );
        for (index, expected) in [(0, A), (1, B)] {
            assert_eq!(
                retained
                    .read_asset(&retained_view, &retained_view.project().files[index])
                    .unwrap(),
                expected
            );
        }

        if point == Point::BeforeSqlCommit {
            let interrupted = tree(target_root.path());
            assert!(FolderSession::open(target_root.path()).await.is_err());
            assert_eq!(tree(target_root.path()), interrupted);
        } else {
            verify_target(
                target_root.path(),
                &original.project().id,
                &expected_history,
            )
            .await;
        }

        let retry_backup = tempfile::tempdir().unwrap();
        let retry_target = tempfile::tempdir().unwrap();
        let backup = source
            .backup_committed(&original, retry_backup.path())
            .unwrap();
        let report = migrate_verified_v2(&source, &original, &backup, retry_target.path()).await;
        assert!(matches!(
            report.target.unwrap(),
            CommitOutcome::Committed(_)
        ));
        verify_target(
            retry_target.path(),
            &original.project().id,
            &expected_history,
        )
        .await;
        assert_eq!(tree(source_root.path()), source_before);
        assert_eq!(tree(backup_root.path()), backup_before);
    }
}

async fn verify_target(path: &Path, project_id: &str, history: &[u8]) {
    let target = FolderSession::open(path).await.unwrap();
    let view = target.capture().await.unwrap();
    assert_eq!(view.project().id, project_id);
    let restored = target.read_history(&view).await.unwrap();
    crate::project_history::validate_archive(view.project(), &restored).unwrap();
    // The SQL export serializes its typed archive in field declaration order.
    // The source fixture uses JSON map order. Compare every field and event.
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&restored).unwrap(),
        serde_json::from_slice::<serde_json::Value>(history).unwrap()
    );
    for (index, expected) in [(0, A), (1, B)] {
        assert_eq!(
            target
                .read_asset(&view, &view.project().files[index])
                .await
                .unwrap(),
            expected
        );
    }
}
