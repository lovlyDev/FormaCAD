//! History authority and persistence fixtures, not CAD geometry validation.
use super::*;
use crate::models::{Project, ProjectFile};
use std::{fs, path::Path};

fn file(name: &str, bytes: &[u8]) -> ProjectFile {
    ProjectFile {
        name: name.into(),
        size: bytes.len() as u64,
        kind: "model".into(),
        data: None,
        sha256: Some(crate::artifacts::digest(bytes)),
    }
}
async fn fixture(root: &Path) -> FolderSession {
    let revision = uuid::Uuid::new_v4().to_string();
    let mut project:Project=serde_json::from_value(serde_json::json!({
        "schemaVersion":1,"id":uuid::Uuid::new_v4().to_string(),"name":"folder history fixture",
        "units":"mm","agent":"codex","pinned":false,"createdAt":"2026-11-01T00:00:00Z",
        "updatedAt":"2026-11-01T00:00:00Z","currentRevision":revision,"messages":[],"exports":[],"files":[],
        "revisions":[{"id":revision,"parent":null,"createdAt":"2026-11-01T00:00:00Z","prompt":"synthetic state A","source":"a.step",
        "parameters":{"kind":"box","width":40,"depth":20,"height":10,"thickness":2,"holeDiameter":0,"holes":0}}]
    })).unwrap();
    let bytes = b"synthetic artifact A";
    project.files.push(file("a.step", bytes));
    let hash = crate::artifacts::digest(bytes);
    assert!(matches!(
        crate::project_repository::sqlite_folder::creation::create(
            root,
            project,
            vec![(hash, bytes.to_vec())],
            None
        )
        .await
        .unwrap(),
        format::CommitOutcome::Committed(_)
    ));
    let session = FolderSession::open(root).await.unwrap();
    let original = session.capture().await.unwrap();
    let mut next = original.project().clone();
    let mut revision = next.revisions[0].clone();
    revision.id = uuid::Uuid::new_v4().to_string();
    revision.parent = next.current_revision.clone();
    revision.parameters.width = 80.0;
    revision.source = Some("b.step".into());
    next.current_revision = Some(revision.id.clone());
    next.revisions.push(revision);
    let bytes = b"synthetic artifact B";
    next.files.push(file("b.step", bytes));
    assert!(matches!(
        session
            .commit(
                &original,
                next,
                vec![(crate::artifacts::digest(bytes), bytes.to_vec())],
                None
            )
            .await
            .unwrap(),
        format::CommitOutcome::Committed(_)
    ));
    session
}

#[tokio::test]
async fn undo_then_fresh_process_read_and_reopened_redo_use_folder_history_and_artifacts() {
    let root = tempfile::tempdir().unwrap();
    let session = fixture(root.path()).await;
    let edited = session.capture().await.unwrap();
    assert!(matches!(
        session
            .apply_history(&edited, Direction::Undo)
            .await
            .unwrap(),
        format::CommitOutcome::Committed(_)
    ));
    let undone = session.capture().await.unwrap();
    let revision = undone
        .project()
        .revisions
        .iter()
        .find(|r| Some(&r.id) == undone.project().current_revision.as_ref())
        .unwrap();
    assert_eq!(revision.parameters.width, 40.0);
    assert_eq!(revision.source.as_deref(), Some("a.step"));
    assert!(undone.saved.history_state().can_redo());
    let nonce = uuid::Uuid::new_v4().to_string();
    let expected = undone.project().current_revision.clone().unwrap();
    session.close();
    drop(session);
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "project_repository::sqlite_folder::session::history::tests::history_child",
            "--nocapture",
        ])
        .env("FORMA_FOLDER_HISTORY_ROOT", root.path())
        .env("FORMA_FOLDER_HISTORY_REVISION", expected)
        .env("FORMA_FOLDER_HISTORY_NONCE", &nonce)
        .output()
        .unwrap();
    assert!(
        child.status.success(),
        "{}",
        String::from_utf8_lossy(&child.stderr)
    );
    let marker = format!("FOLDER_HISTORY_OK:{nonce}");
    assert!(String::from_utf8_lossy(&child.stdout)
        .lines()
        .any(|line| line == marker));
    let reopened = FolderSession::open(root.path()).await.unwrap();
    let current = reopened.capture().await.unwrap();
    assert!(matches!(
        reopened
            .apply_history(&current, Direction::Redo)
            .await
            .unwrap(),
        format::CommitOutcome::Committed(_)
    ));
    let redone = reopened.capture().await.unwrap();
    let revision = redone
        .project()
        .revisions
        .iter()
        .find(|r| Some(&r.id) == redone.project().current_revision.as_ref())
        .unwrap();
    assert_eq!(revision.parameters.width, 80.0);
    assert_eq!(revision.source.as_deref(), Some("b.step"));
    assert!(!redone.saved.history_state().can_redo());
    assert_eq!(
        reopened
            .read_asset(&redone, &redone.project().files[1])
            .await
            .unwrap(),
        b"synthetic artifact B"
    );
}

#[tokio::test]
#[ignore = "exact child-process folder history probe"]
async fn history_child() {
    let Some(root) = std::env::var_os("FORMA_FOLDER_HISTORY_ROOT") else {
        return;
    };
    let session = FolderSession::open(Path::new(&root)).await.unwrap();
    let saved = session.capture().await.unwrap();
    let expected = std::env::var("FORMA_FOLDER_HISTORY_REVISION").unwrap();
    assert_eq!(
        saved.project().current_revision.as_deref(),
        Some(expected.as_str())
    );
    assert!(saved.saved.history_state().can_redo());
    assert_eq!(
        session
            .read_asset(&saved, &saved.project().files[0])
            .await
            .unwrap(),
        b"synthetic artifact A"
    );
    println!(
        "\nFOLDER_HISTORY_OK:{}",
        std::env::var("FORMA_FOLDER_HISTORY_NONCE").unwrap()
    );
}

#[tokio::test]
async fn stale_or_closed_history_snapshot_cannot_publish_or_change_saved_bytes() {
    let root = tempfile::tempdir().unwrap();
    let session = fixture(root.path()).await;
    let original = session.capture().await.unwrap();
    let mut renamed = original.project().clone();
    renamed.name = "latest metadata".into();
    assert!(matches!(
        session
            .commit(&original, renamed, vec![], None)
            .await
            .unwrap(),
        format::CommitOutcome::Committed(_)
    ));
    let before = fs::read(root.path().join("project.sqlite")).unwrap();
    assert!(session
        .apply_history(&original, Direction::Undo)
        .await
        .is_err());
    let latest = session.capture().await.unwrap();
    session.close();
    assert!(session
        .apply_history(&latest, Direction::Undo)
        .await
        .is_err());
    assert_eq!(
        fs::read(root.path().join("project.sqlite")).unwrap(),
        before
    );
}
