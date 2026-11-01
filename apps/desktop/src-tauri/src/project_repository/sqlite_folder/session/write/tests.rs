//! Synthetic standalone storage fixtures; these are not real CAD commits.
use super::super::super::creation;
use super::*;
use crate::models::ProjectFile;
use std::{fs, time::Duration};

fn project() -> Project {
    serde_json::from_value(serde_json::json!({
        "schemaVersion":1,"id":uuid::Uuid::new_v4().to_string(),"name":"session publication fixture",
        "units":"mm","agent":"codex","pinned":false,"createdAt":"2026-11-01T00:00:00Z",
        "updatedAt":"2026-11-01T00:00:00Z","currentRevision":null,"messages":[],
        "exports":[],"files":[],"revisions":[]
    })).unwrap()
}
async fn fixture(root: &std::path::Path) -> FolderSession {
    assert!(matches!(
        creation::create(root, project(), vec![], None)
            .await
            .unwrap(),
        format::CommitOutcome::Committed(_)
    ));
    FolderSession::open(root).await.unwrap()
}

#[tokio::test]
async fn pending_asset_commit_advances_the_same_opening_and_retains_history() {
    let root = tempfile::tempdir().unwrap();
    let session = fixture(root.path()).await;
    let original = session.capture().await.unwrap();
    let history = session.read_history(&original).await.unwrap();
    let mut next = original.project().clone();
    next.name = "published metadata and immutable asset".into();
    let bytes = b"synthetic STEP-shaped storage fixture".to_vec();
    let hash = crate::artifacts::digest(&bytes);
    next.files.push(ProjectFile {
        name: "fixture.step".into(),
        size: bytes.len() as u64,
        kind: "model".into(),
        data: None,
        sha256: Some(hash.clone()),
    });
    let result = session
        .commit(&original, next.clone(), vec![(hash, bytes.clone())], None)
        .await
        .unwrap();
    assert!(matches!(result, format::CommitOutcome::Committed(_)));
    assert!(session.recheck(&original).await.is_err());
    let saved = session.capture().await.unwrap();
    assert_eq!(saved.project().name, next.name);
    assert_eq!(
        session.read_asset(&saved, &next.files[0]).await.unwrap(),
        bytes
    );
    assert_eq!(session.read_history(&saved).await.unwrap(), history);
    assert!(!root.path().join("project.sqlite-journal").exists());
}

#[tokio::test]
async fn metadata_continuation_produces_a_fresh_exact_expectation_for_publication() {
    let root = tempfile::tempdir().unwrap();
    let session = fixture(root.path()).await;
    let original = session.capture().await.unwrap();
    let mut renamed = original.project().clone();
    renamed.name = "concurrent metadata".into();
    assert!(matches!(
        session
            .commit(&original, renamed, vec![], None)
            .await
            .unwrap(),
        format::CommitOutcome::Committed(_)
    ));
    let latest = session.latest_compatible(&original).await.unwrap();
    assert_eq!(latest.project().name, "concurrent metadata");
    let mut pinned = latest.project().clone();
    pinned.pinned = true;
    assert!(matches!(
        session.commit(&latest, pinned, vec![], None).await.unwrap(),
        format::CommitOutcome::Committed(_)
    ));
    let saved = session.capture().await.unwrap();
    assert_eq!(saved.project().name, "concurrent metadata");
    assert!(saved.project().pinned);
    session.close();
    assert!(session.latest_compatible(&original).await.is_err());
}

#[tokio::test]
async fn stale_cross_opening_busy_and_invalid_inputs_do_not_change_saved_sql() {
    let root = tempfile::tempdir().unwrap();
    let session = fixture(root.path()).await;
    let original = session.capture().await.unwrap();
    let other = FolderSession::open(root.path()).await.unwrap();
    assert!(other
        .commit(&original, original.project().clone(), vec![], None)
        .await
        .is_err());
    let before = fs::read(root.path().join("project.sqlite")).unwrap();
    let busy = FolderLease::acquire(root.path()).unwrap();
    assert!(session
        .commit(&original, original.project().clone(), vec![], None)
        .await
        .is_err());
    drop(busy);
    let mut wrong = original.project().clone();
    wrong.id = uuid::Uuid::new_v4().to_string();
    assert!(session
        .commit(&original, wrong, vec![], None)
        .await
        .is_err());
    assert!(session
        .commit(
            &original,
            original.project().clone(),
            vec![("0".repeat(64), vec![1])],
            None
        )
        .await
        .is_err());
    assert_eq!(
        before,
        fs::read(root.path().join("project.sqlite")).unwrap()
    );
    let mut next = original.project().clone();
    next.name = "advanced".into();
    assert!(matches!(
        session.commit(&original, next, vec![], None).await.unwrap(),
        format::CommitOutcome::Committed(_)
    ));
    assert!(session
        .commit(&original, original.project().clone(), vec![], None)
        .await
        .is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancelled_blocking_publication_keeps_lease_until_job_finishes() {
    let root = tempfile::tempdir().unwrap();
    let session = Arc::new(fixture(root.path()).await);
    let original = session.capture().await.unwrap();
    let before = fs::read(root.path().join("project.sqlite")).unwrap();
    let (entered, waiting) = tokio::sync::oneshot::channel();
    let (release, ready) = std::sync::mpsc::sync_channel(1);
    let worker = Arc::clone(&session);
    let task = tokio::spawn(async move {
        let next = original.project().clone();
        worker
            .commit_inner(
                &original,
                next,
                vec![],
                None,
                Some(Box::new(move || {
                    let _ = entered.send(());
                    ready.recv_timeout(Duration::from_secs(5)).unwrap();
                })),
            )
            .await
    });
    tokio::time::timeout(Duration::from_secs(5), waiting)
        .await
        .unwrap()
        .unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    assert!(FolderLease::acquire(root.path()).is_err());
    release.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if FolderLease::acquire(root.path()).is_ok() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        before,
        fs::read(root.path().join("project.sqlite")).unwrap()
    );
    session.capture().await.unwrap();
}

#[tokio::test]
async fn commit_future_is_send() {
    fn assert_send<T: Send>(_: T) {}
    let root = tempfile::tempdir().unwrap();
    let session = fixture(root.path()).await;
    let original = session.capture().await.unwrap();
    assert_send(session.commit(&original, original.project().clone(), vec![], None));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn close_while_publication_is_queued_refuses_sql_and_releases_lease() {
    let root = tempfile::tempdir().unwrap();
    let session = Arc::new(fixture(root.path()).await);
    let original = session.capture().await.unwrap();
    let before = fs::read(root.path().join("project.sqlite")).unwrap();
    let (entered, waiting) = tokio::sync::oneshot::channel();
    let (release, ready) = std::sync::mpsc::sync_channel(1);
    let worker = Arc::clone(&session);
    let task = tokio::spawn(async move {
        let next = original.project().clone();
        worker
            .commit_inner(
                &original,
                next,
                vec![],
                None,
                Some(Box::new(move || {
                    let _ = entered.send(());
                    ready.recv_timeout(Duration::from_secs(5)).unwrap();
                })),
            )
            .await
    });
    tokio::time::timeout(Duration::from_secs(5), waiting)
        .await
        .unwrap()
        .unwrap();
    session.close();
    assert!(FolderLease::acquire(root.path()).is_err());
    release.send(()).unwrap();
    assert!(task.await.unwrap().is_err());
    assert!(FolderLease::acquire(root.path()).is_ok());
    assert_eq!(
        before,
        fs::read(root.path().join("project.sqlite")).unwrap()
    );
}
