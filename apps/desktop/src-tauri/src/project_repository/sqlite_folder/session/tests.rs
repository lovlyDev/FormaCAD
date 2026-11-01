//! Real standalone database fixtures, without AppState or managed app SQL.
#![cfg(any(target_os = "linux", windows))]
use super::*;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode},
    Connection, SqliteConnection,
};

fn project() -> Project {
    serde_json::from_value(serde_json::json!({
        "schemaVersion":1,"id":uuid::Uuid::new_v4().to_string(),"name":"session fixture",
        "units":"mm","agent":"codex","pinned":false,"createdAt":"2026-11-01T00:00:00Z",
        "updatedAt":"2026-11-01T00:00:00Z","currentRevision":null,"messages":[],
        "exports":[],"files":[],"revisions":[]
    }))
    .unwrap()
}

async fn writer(root: &Path) -> SqliteConnection {
    SqliteConnection::connect_with(
        &SqliteConnectOptions::new()
            .filename(root.join("project.sqlite"))
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(SqliteJournalMode::Delete),
    )
    .await
    .unwrap()
}

async fn fixture(root: &Path) {
    std::fs::create_dir(root.join("assets")).unwrap();
    let mut writer = writer(root).await;
    let result = format::initialize(
        &mut writer,
        &uuid::Uuid::new_v4().to_string(),
        &project(),
        None,
    )
    .await
    .unwrap();
    assert!(matches!(result, format::CommitOutcome::Committed(_)));
    writer.close().await.unwrap();
}

#[tokio::test]
async fn copied_database_and_reopening_never_accept_the_previous_opening_snapshot() {
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    fixture(a.path()).await;
    std::fs::create_dir(b.path().join("assets")).unwrap();
    std::fs::copy(
        a.path().join("project.sqlite"),
        b.path().join("project.sqlite"),
    )
    .unwrap();
    let first = FolderSession::open(a.path()).await.unwrap();
    let copied = FolderSession::open(b.path()).await.unwrap();
    let snapshot = first.capture().await.unwrap();
    assert_eq!(
        copied.capture().await.unwrap().project().id,
        snapshot.project().id
    );
    assert!(copied.recheck(&snapshot).await.is_err());
    assert!(!first.read_history(&snapshot).await.unwrap().is_empty());
    first.close();
    assert!(first.recheck(&snapshot).await.is_err());
    let reopened = FolderSession::open(a.path()).await.unwrap();
    assert!(reopened.recheck(&snapshot).await.is_err());
    let current = reopened.capture().await.unwrap();
    reopened.recheck(&current).await.unwrap();
    let forged = ProjectFile {
        name: "not-in-snapshot.step".into(),
        size: 1,
        kind: "step".into(),
        data: None,
        sha256: Some("0".repeat(64)),
    };
    assert!(reopened.read_asset(&current, &forged).await.is_err());
}

#[tokio::test]
async fn committed_metadata_invalidates_old_saved_state_without_invalidating_opening() {
    let temp = tempfile::tempdir().unwrap();
    fixture(temp.path()).await;
    let session = FolderSession::open(temp.path()).await.unwrap();
    let before = session.capture().await.unwrap();
    let mut next = before.project().clone();
    next.name = "committed name".into();
    let mut writer = writer(temp.path()).await;
    let result = format::commit(&mut writer, &before.saved, &next, None)
        .await
        .unwrap();
    assert!(matches!(result, format::CommitOutcome::Committed(_)));
    writer.close().await.unwrap();
    assert!(session.recheck(&before).await.is_err());
    assert_eq!(
        session
            .resolve_commit(&before, before.saved.expectation().commit_id())
            .await
            .unwrap(),
        format::CommitPresence::Present { is_head: false }
    );
    let after = session.capture().await.unwrap();
    assert_eq!(after.project().name, next.name);
    session.recheck(&after).await.unwrap();
    session.close();
    assert!(session
        .resolve_commit(&before, before.saved.expectation().commit_id())
        .await
        .is_err());
}

#[tokio::test]
async fn closing_while_capture_waits_for_connection_gate_rejects_late_result() {
    let temp = tempfile::tempdir().unwrap();
    fixture(temp.path()).await;
    let session = Arc::new(FolderSession::open(temp.path()).await.unwrap());
    let gate = session.database.lock().await;
    let mut capture = Box::pin(session.capture());
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(20), &mut capture)
            .await
            .is_err()
    );
    session.close();
    drop(gate);
    assert!(capture.await.is_err());
}

#[tokio::test]
async fn asset_reads_require_captured_descriptor_and_recheck_content() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::create_dir(temp.path().join("assets")).unwrap();
    let bytes = b"synthetic CAS content; no CAD validity claim";
    let hash = crate::artifacts::digest(bytes);
    std::fs::write(temp.path().join("assets").join(&hash), bytes).unwrap();
    let descriptor = ProjectFile {
        name: "fixture.step".into(),
        size: bytes.len() as u64,
        kind: "step".into(),
        data: None,
        sha256: Some(hash.clone()),
    };
    let mut project = project();
    project.files.push(descriptor.clone());
    let mut writer = writer(temp.path()).await;
    assert!(matches!(
        format::initialize(
            &mut writer,
            &uuid::Uuid::new_v4().to_string(),
            &project,
            None
        )
        .await
        .unwrap(),
        format::CommitOutcome::Committed(_)
    ));
    writer.close().await.unwrap();
    let session = FolderSession::open(temp.path()).await.unwrap();
    let snapshot = session.capture().await.unwrap();
    assert_eq!(
        session.read_asset(&snapshot, &descriptor).await.unwrap(),
        bytes
    );
    let mut forged = descriptor.clone();
    forged.name = "another.step".into();
    assert!(session.read_asset(&snapshot, &forged).await.is_err());
    std::fs::write(
        temp.path().join("assets").join(hash),
        vec![b'X'; bytes.len()],
    )
    .unwrap();
    assert!(session.read_asset(&snapshot, &descriptor).await.is_err());
}
