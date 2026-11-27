//! Storage fixtures use synthetic bytes; the real CAD vertical path is separate.
use super::*;
use crate::models::ProjectFile;
use std::{fs, time::Duration};

fn fixture() -> (Project, Vec<(String, Vec<u8>)>) {
    let bytes = b"standalone creation synthetic CAS bytes".to_vec();
    let hash = crate::artifacts::digest(&bytes);
    let mut project: Project = serde_json::from_value(serde_json::json!({
        "schemaVersion":1,"id":uuid::Uuid::new_v4().to_string(),"name":"standalone fixture",
        "units":"mm","agent":"codex","pinned":false,"createdAt":"2026-11-01T00:00:00Z",
        "updatedAt":"2026-11-01T00:00:00Z","currentRevision":null,"messages":[],
        "exports":[],"files":[],"revisions":[]
    }))
    .unwrap();
    project.files.push(ProjectFile {
        name: "fixture.step".into(),
        size: bytes.len() as u64,
        kind: "model".into(),
        data: None,
        sha256: Some(hash.clone()),
    });
    (project, vec![(hash, bytes)])
}

#[tokio::test]
async fn creation_is_readable_in_an_independent_process_without_app_index() {
    let root = tempfile::tempdir().unwrap();
    let (project, pending) = fixture();
    let hash = pending[0].0.clone();
    let id = project.id.clone();
    let outcome = create(root.path(), project, pending, None).await.unwrap();
    let snapshot = match outcome {
        format::CommitOutcome::Committed(snapshot) => snapshot,
        other => panic!("initial publication failed: {other:?}"),
    };
    assert_eq!(snapshot.project().id, id);
    assert_ne!(snapshot.expectation().repository_id(), id);
    let nonce = uuid::Uuid::new_v4().to_string();
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "project_repository::sqlite_folder::creation::tests::reopen_child",
            "--nocapture",
        ])
        .env("FORMA_FCP1_CREATE_ROOT", root.path())
        .env("FORMA_FCP1_CREATE_ID", id)
        .env("FORMA_FCP1_CREATE_HASH", hash)
        .env("FORMA_FCP1_CREATE_NONCE", &nonce)
        .output()
        .unwrap();
    assert!(
        child.status.success(),
        "{}",
        String::from_utf8_lossy(&child.stderr)
    );
    let marker = format!("FCP1_CREATE_REOPEN_OK:{nonce}");
    assert!(String::from_utf8_lossy(&child.stdout)
        .lines()
        .any(|line| line == marker));
    assert!(!root.path().join("project.sqlite-journal").exists());
}

#[tokio::test]
#[ignore = "owned child-process standalone reopening probe"]
async fn reopen_child() {
    let Some(root) = std::env::var_os("FORMA_FCP1_CREATE_ROOT") else {
        return;
    };
    let session = super::super::session::FolderSession::open(Path::new(&root))
        .await
        .unwrap();
    let saved = session.capture().await.unwrap();
    assert_eq!(
        saved.project().id,
        std::env::var("FORMA_FCP1_CREATE_ID").unwrap()
    );
    let file = &saved.project().files[0];
    let bytes = session.read_asset(&saved, file).await.unwrap();
    assert_eq!(
        crate::artifacts::digest(&bytes),
        std::env::var("FORMA_FCP1_CREATE_HASH").unwrap()
    );
    session.recheck(&saved).await.unwrap();
    println!(
        "\nFCP1_CREATE_REOPEN_OK:{}",
        std::env::var("FORMA_FCP1_CREATE_NONCE").unwrap()
    );
}

#[tokio::test]
async fn malformed_batch_and_nonempty_selected_root_are_preserved_without_initialization() {
    let root = tempfile::tempdir().unwrap();
    let (project, mut pending) = fixture();
    pending[0].1[0] ^= 1;
    assert!(create(root.path(), project, pending, None).await.is_err());
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
    let (project, pending) = fixture();
    fs::write(root.path().join("user.step"), b"existing unrelated bytes").unwrap();
    assert!(create(root.path(), project, pending, None).await.is_err());
    assert_eq!(
        fs::read(root.path().join("user.step")).unwrap(),
        b"existing unrelated bytes"
    );
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancelled_publication_retains_lease_until_blocking_job_finishes_without_sql_commit() {
    let root = tempfile::tempdir().unwrap();
    let (project, pending) = fixture();
    let path = root.path().to_path_buf();
    let (entered, waiting) = tokio::sync::oneshot::channel();
    let (release, blocked) = std::sync::mpsc::sync_channel(1);
    let task = tokio::spawn(async move {
        create_inner(
            &path,
            project,
            pending,
            None,
            Some(Box::new(move || {
                entered.send(()).unwrap();
                blocked.recv_timeout(Duration::from_secs(10)).unwrap();
            })),
        )
        .await
    });
    tokio::time::timeout(Duration::from_secs(10), waiting)
        .await
        .unwrap()
        .unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    assert!(FolderLease::acquire(root.path()).is_err());
    release.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if let Ok(lease) = FolderLease::acquire(root.path()) {
                drop(lease);
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert!(!root.path().join("project.sqlite").exists());
}
