use super::*;
use crate::project_repository::sqlite_folder::api::SavedCommit;

fn committed(outcome: Publication) -> SavedCommit {
    match outcome {
        Publication::Committed(saved) => saved,
        _ => panic!("expected a confirmed publication"),
    }
}

async fn fixture(root: &Path, id: &str, name: &str) {
    let project: Project = serde_json::from_value(serde_json::json!({
        "schemaVersion":1,"id":id,"name":name,"units":"mm","agent":"codex","pinned":false,
        "createdAt":"2026-11-05T00:00:00Z","updatedAt":"2026-11-05T00:00:00Z","currentRevision":null,
        "messages":[],"exports":[],"files":[],"revisions":[]
    })).unwrap();
    committed(
        FolderProject::create(root, project, vec![], None)
            .await
            .unwrap(),
    );
}

#[tokio::test]
async fn operation_keeps_window_source_and_saved_state_together_without_read_grants() {
    let first_root = tempfile::tempdir().unwrap();
    let second_root = tempfile::tempdir().unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    fixture(first_root.path(), &id, "First source").await;
    fixture(second_root.path(), &id, "Second source").await;
    let windows = WindowSources::default();
    let scope = windows.for_window("first").unwrap();
    windows.for_window("second").unwrap();
    let first = scope.open(first_root.path()).await.unwrap();
    let second = scope.open(second_root.path()).await.unwrap();
    let key = first.id.to_string();
    let commit = first.snapshot.commit_id();
    let before = std::fs::read(first_root.path().join("project.sqlite")).unwrap();
    let operation = windows
        .capture_operation("first", &key, &id, commit)
        .await
        .unwrap();
    assert_eq!(operation.project().name, "First source");
    assert_eq!(operation.commit_id(), commit);
    assert_eq!(operation.task_scope_id(), first.snapshot.task_scope_id());
    assert!(windows
        .capture_operation("second", &key, &id, commit)
        .await
        .is_err());
    assert!(windows
        .capture_operation("first", &key, "wrong-id", commit)
        .await
        .is_err());
    assert!(windows
        .capture_operation("first", &key, &id, "wrong-commit")
        .await
        .is_err());
    let foreign = windows
        .capture_operation(
            "first",
            &second.id.to_string(),
            &id,
            second.snapshot.commit_id(),
        )
        .await
        .unwrap();
    let foreign_approval = foreign.approve_write().await.unwrap();
    let candidate = operation.project().clone();
    // An actual opaque approval from the wrong original opening still fails.
    assert!(operation
        .commit(foreign_approval, candidate, vec![])
        .await
        .is_err());
    assert_eq!(
        before,
        std::fs::read(first_root.path().join("project.sqlite")).unwrap()
    );
    let reader = foreign.into_read().await.unwrap();
    assert_eq!(reader.project().name, "Second source");
    reader.recheck().await.unwrap();
}

#[tokio::test]
async fn approved_operation_cannot_write_after_close_or_reused_window_label() {
    let root = tempfile::tempdir().unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    fixture(root.path(), &id, "Original").await;
    let windows = WindowSources::default();
    let scope = windows.for_window("window").unwrap();
    let opened = scope.open(root.path()).await.unwrap();
    let key = opened.id.to_string();
    let commit = opened.snapshot.commit_id();
    let original = windows
        .capture_operation("window", &key, &id, commit)
        .await
        .unwrap();
    let approval = original.approve_write().await.unwrap();
    let mut candidate = original.project().clone();
    candidate.name = "Must not be written".into();
    let before = std::fs::read(root.path().join("project.sqlite")).unwrap();
    windows.close_window("window");
    let replacement = windows.for_window("window").unwrap();
    let reopened = replacement.open(root.path()).await.unwrap();
    assert_ne!(key, reopened.id.to_string());
    assert!(original.recheck().await.is_err());
    assert!(original.commit(approval, candidate, vec![]).await.is_err());
    assert!(windows
        .capture_operation("window", &key, &id, commit)
        .await
        .is_err());
    assert_eq!(
        before,
        std::fs::read(root.path().join("project.sqlite")).unwrap()
    );
    let current = windows
        .capture_operation(
            "window",
            &reopened.id.to_string(),
            &id,
            reopened.snapshot.commit_id(),
        )
        .await
        .unwrap();
    assert_eq!(current.project().name, "Original");
}

#[tokio::test]
async fn delayed_approval_never_recaptures_a_newer_commit_to_legalize_an_old_candidate() {
    let root = tempfile::tempdir().unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    fixture(root.path(), &id, "Original").await;
    let windows = WindowSources::default();
    let scope = windows.for_window("window").unwrap();
    let opened = scope.open(root.path()).await.unwrap();
    let key = opened.id.to_string();
    let first = windows
        .capture_operation("window", &key, &id, opened.snapshot.commit_id())
        .await
        .unwrap();
    let pending = windows
        .capture_operation("window", &key, &id, opened.snapshot.commit_id())
        .await
        .unwrap();
    let approval = first.approve_write().await.unwrap();
    let mut newer = first.project().clone();
    newer.name = "Newly committed".into();
    let saved = committed(first.commit(approval, newer, vec![]).await.unwrap());
    assert_ne!(saved.commit_id(), opened.snapshot.commit_id());
    let before = std::fs::read(root.path().join("project.sqlite")).unwrap();
    assert!(pending.approve_write().await.is_err());
    assert!(pending.recheck().await.is_err());
    assert_eq!(
        before,
        std::fs::read(root.path().join("project.sqlite")).unwrap()
    );
    let current = windows
        .capture_operation("window", &key, &id, saved.commit_id())
        .await
        .unwrap();
    assert_eq!(current.project().name, "Newly committed");
}

#[tokio::test]
async fn window_read_only_copy_does_not_acquire_source_writer_authority() {
    let root = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    fixture(root.path(), &id, "Read-only source").await;
    let windows = WindowSources::default();
    let scope = windows.for_window("window").unwrap();
    let opened = scope.open(root.path()).await.unwrap();
    let operation = windows
        .capture_operation(
            "window",
            &opened.id.to_string(),
            &id,
            opened.snapshot.commit_id(),
        )
        .await
        .unwrap();
    let before = std::fs::read(root.path().join("project.sqlite")).unwrap();
    let lease = crate::project_access::folder_lease::FolderLease::acquire(root.path()).unwrap();
    let copied = committed(operation.save_copy(target.path()).await.unwrap());
    assert_ne!(copied.project().id, id);
    assert_eq!(
        before,
        std::fs::read(root.path().join("project.sqlite")).unwrap()
    );
    drop(lease);
    let copy = FolderProject::open(target.path()).await.unwrap();
    let snapshot = copy.capture().await.unwrap();
    assert_eq!(snapshot.project().id, copied.project().id);
    assert_eq!(snapshot.project().name, "Read-only source");
}
