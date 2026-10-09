use super::*;
use crate::{core::AppState, models::Project};
use std::{
    path::Path,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

fn project() -> Project {
    serde_json::from_value(serde_json::json!({"schemaVersion":1,"id":uuid::Uuid::new_v4().to_string(),"name":"Lease fixture","units":"mm","agent":"codex","pinned":false,"createdAt":"2026-10-04T00:00:00Z","updatedAt":"2026-10-04T00:00:00Z","revisions":[],"currentRevision":null,"messages":[],"files":[],"exports":[]})).unwrap()
}
async fn state(root: &Path) -> AppState {
    let pool = crate::storage::open(&root.join("fixture.sqlite"))
        .await
        .unwrap();
    let (_, guard) = tracing_appender::non_blocking(std::io::sink());
    AppState {
        cad_tasks: Default::default(),
        root: root.into(),
        pool,
        project_access: Default::default(),
        grants: Default::default(),
        tasks: Default::default(),
        writes: Default::default(),
        _log_guard: guard,
    }
}

#[test]
fn second_registry_is_read_only_until_explicit_reacquire() {
    let root = tempfile::tempdir().unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    let owner = AccessRegistry::default();
    let second = AccessRegistry::default();
    assert_eq!(second.status(root.path(), &id).unwrap().mode, "closed");
    assert_eq!(owner.acquire(root.path(), &id).unwrap().mode, "write");
    assert_eq!(second.acquire(root.path(), &id).unwrap().mode, "read_only");
    assert!(second
        .ensure_write(root.path(), &id)
        .unwrap_err()
        .to_string()
        .contains("PROJECT_READ_ONLY"));
    owner.release(&id).unwrap();
    assert!(second.ensure_write(root.path(), &id).is_err());
    assert_eq!(second.acquire(root.path(), &id).unwrap().mode, "write");
    second.ensure_write(root.path(), &id).unwrap();
}

#[tokio::test]
async fn read_only_save_rejects_before_any_artifact_or_index_change_and_copy_succeeds() {
    let root = tempfile::tempdir().unwrap();
    let first = state(root.path()).await;
    let second = state(root.path()).await;
    let p = project();
    crate::projects::persist(&first, p.clone(), Vec::new(), None)
        .await
        .unwrap();
    assert_eq!(
        first
            .project_access
            .status(&first.root, &p.id)
            .unwrap()
            .mode,
        "closed"
    );
    first.project_access.acquire(&first.root, &p.id).unwrap();
    assert_eq!(
        second
            .project_access
            .status(&second.root, &p.id)
            .unwrap()
            .mode,
        "closed"
    );
    crate::projects::get(&second, &p.id).await.unwrap();
    assert_eq!(
        second
            .project_access
            .status(&second.root, &p.id)
            .unwrap()
            .mode,
        "closed"
    );
    assert_eq!(
        second
            .project_access
            .acquire(&second.root, &p.id)
            .unwrap()
            .mode,
        "read_only"
    );
    let path = root.path().join(&p.id);
    let before = std::fs::read(path.join("manifest.json")).unwrap();
    let mut edit = p.clone();
    edit.name = "Rejected".into();
    assert!(crate::projects::persist(&second, edit, Vec::new(), None)
        .await
        .unwrap_err()
        .to_string()
        .contains("PROJECT_READ_ONLY"));
    assert_eq!(std::fs::read(path.join("manifest.json")).unwrap(), before);
    assert_eq!(
        crate::projects::get(&first, &p.id).await.unwrap().name,
        p.name
    );
    let copied = crate::projects::import_folder(&second, &path, true, None)
        .await
        .unwrap();
    assert_ne!(copied.id, p.id);
    assert_eq!(copied.name, p.name);
    assert_eq!(std::fs::read(path.join("manifest.json")).unwrap(), before);
}

#[tokio::test]
async fn read_only_permission_guard_precedes_grant_and_audit_mutations() {
    let root = tempfile::tempdir().unwrap();
    let first = state(root.path()).await;
    let second = state(root.path()).await;
    let id = uuid::Uuid::new_v4().to_string();
    first.project_access.acquire(&first.root, &id).unwrap();
    second.project_access.acquire(&second.root, &id).unwrap();
    for action in ["modify_project", "run_agent", "convert_file"] {
        assert!(crate::permissions::consume(&second, &id, action)
            .await
            .unwrap_err()
            .to_string()
            .contains("PROJECT_READ_ONLY"));
    }
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM permissions")
        .fetch_one(&second.pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
#[ignore = "subprocess-only helper; parent runs it with an isolated temporary root"]
fn child_process_writer() {
    let root = std::env::var_os("FORMA_LEASE_TEST_ROOT").unwrap();
    let id = std::env::var("FORMA_LEASE_TEST_ID").unwrap();
    let registry = AccessRegistry::default();
    assert_eq!(
        registry.acquire(Path::new(&root), &id).unwrap().mode,
        "write"
    );
    std::fs::write(Path::new(&root).join("ready"), b"ready").unwrap();
    std::thread::sleep(Duration::from_secs(60));
}
struct ChildGuard(Child);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn actual_second_process_cannot_write_and_crash_releases_os_lease() {
    let root = tempfile::tempdir().unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    let mut child = ChildGuard(
        Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "project_access::tests::child_process_writer",
                "--ignored",
                "--nocapture",
            ])
            .env("FORMA_LEASE_TEST_ROOT", root.path())
            .env("FORMA_LEASE_TEST_ID", &id)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let start = Instant::now();
    while !root.path().join("ready").exists() {
        assert!(
            start.elapsed() < Duration::from_secs(10),
            "child did not acquire lease"
        );
        assert!(
            child.0.try_wait().unwrap().is_none(),
            "child exited unexpectedly"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    let registry = AccessRegistry::default();
    let access = registry.acquire(root.path(), &id).unwrap();
    assert_eq!(access.mode, "read_only");
    assert_eq!(access.owner_pid, Some(child.0.id()));
    assert!(registry.ensure_write(root.path(), &id).is_err());
    child.0.kill().unwrap();
    child.0.wait().unwrap();
    assert_eq!(registry.acquire(root.path(), &id).unwrap().mode, "write");
}

#[test]
fn scoped_mutations_do_not_own_closed_projects_and_nested_guards_share_lease() {
    let root = tempfile::tempdir().unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    let first = AccessRegistry::default();
    let second = AccessRegistry::default();
    let outer = first.ensure_write(root.path(), &id).unwrap();
    let inner = first.ensure_write(root.path(), &id).unwrap();
    assert_eq!(first.status(root.path(), &id).unwrap().mode, "closed");
    assert_eq!(second.acquire(root.path(), &id).unwrap().mode, "read_only");
    drop(outer);
    assert_eq!(second.acquire(root.path(), &id).unwrap().mode, "read_only");
    drop(inner);
    assert_eq!(second.acquire(root.path(), &id).unwrap().mode, "write");
}

#[tokio::test]
async fn copy_for_editing_preserves_read_only_source_and_does_not_acquire_original() {
    let root = tempfile::tempdir().unwrap();
    let first = state(root.path()).await;
    let second = state(root.path()).await;
    let p = project();
    crate::projects::persist(&first, p.clone(), Vec::new(), None)
        .await
        .unwrap();
    first.project_access.acquire(&first.root, &p.id).unwrap();
    second.project_access.acquire(&second.root, &p.id).unwrap();
    let before = std::fs::read(root.path().join(&p.id).join("manifest.json")).unwrap();
    let copied = copy::copy_for_editing(&second, &p.id).await.unwrap();
    assert_ne!(copied.id, p.id);
    assert_eq!(copied.current_revision, p.current_revision);
    assert_eq!(
        second
            .project_access
            .status(&second.root, &p.id)
            .unwrap()
            .mode,
        "read_only"
    );
    assert_eq!(
        second
            .project_access
            .status(&second.root, &copied.id)
            .unwrap()
            .mode,
        "closed"
    );
    assert_eq!(
        std::fs::read(root.path().join(&p.id).join("manifest.json")).unwrap(),
        before
    );
}
