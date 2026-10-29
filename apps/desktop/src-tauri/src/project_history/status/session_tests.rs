//! Repository lifecycle regressions through the actual undo/redo status consumer.
use super::{from_snapshot, status_for};
use crate::{
    project_access::AccessRegistry,
    project_repository::{self as repository, legacy::read_test_fixture as fixture},
};
use std::{future::Future, task::Poll};

#[tokio::test]
async fn close_while_sql_recheck_waits_rejects_original_history_without_writes() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let project = fixture::project(&state, 16).await;
    let snapshot = repository::capture_history_state(&state, &project.id)
        .await
        .unwrap();
    let before = fixture::logical(&state, &project.id).await;
    let files = fixture::files(temp.path());
    let mut held = Vec::new();
    for _ in 0..state.pool.options().get_max_connections() {
        held.push(state.pool.acquire().await.unwrap());
    }
    let checking = repository::recheck_history_state(&state, &snapshot);
    tokio::pin!(checking);
    std::future::poll_fn(|context| {
        assert!(checking.as_mut().poll(context).is_pending());
        Poll::Ready(())
    })
    .await;
    state.project_access.release(&project.id).unwrap();
    drop(held);
    assert_eq!(
        checking.await.err().unwrap().to_string(),
        "PROJECT_SNAPSHOT_CHANGED"
    );
    assert_eq!(fixture::logical(&state, &project.id).await, before);
    assert_eq!(fixture::files(temp.path()), files);
}

#[tokio::test]
async fn close_while_sql_capture_waits_rejects_the_new_snapshot_without_writes() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let project = fixture::project(&state, 16).await;
    let before = fixture::logical(&state, &project.id).await;
    let files = fixture::files(temp.path());
    let mut held = Vec::new();
    for _ in 0..state.pool.options().get_max_connections() {
        held.push(state.pool.acquire().await.unwrap());
    }
    let capture = repository::capture_history_state(&state, &project.id);
    tokio::pin!(capture);
    // Poll once with every SQL connection held: the session has been captured,
    // and its SQL read cannot yet complete. No timing or scheduler assumption.
    std::future::poll_fn(|context| {
        assert!(capture.as_mut().poll(context).is_pending());
        Poll::Ready(())
    })
    .await;
    state.project_access.release(&project.id).unwrap();
    drop(held);
    assert_eq!(
        capture.await.err().unwrap().to_string(),
        "PROJECT_SNAPSHOT_CHANGED"
    );
    assert_eq!(fixture::logical(&state, &project.id).await, before);
    assert_eq!(fixture::files(temp.path()), files);
}

#[tokio::test]
async fn close_and_reopen_reject_old_history_even_when_source_bytes_are_unchanged() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let project = fixture::project(&state, 16).await;
    state
        .project_access
        .acquire(&state.root, &project.id)
        .unwrap();
    let snapshot = repository::capture_history_state(&state, &project.id)
        .await
        .unwrap();
    let before = fixture::logical(&state, &project.id).await;
    state.project_access.release(&project.id).unwrap();
    assert_eq!(
        from_snapshot(&state, &snapshot)
            .await
            .err()
            .unwrap()
            .to_string(),
        "PROJECT_SNAPSHOT_CHANGED"
    );
    state
        .project_access
        .acquire(&state.root, &project.id)
        .unwrap();
    assert_eq!(
        from_snapshot(&state, &snapshot)
            .await
            .err()
            .unwrap()
            .to_string(),
        "PROJECT_SNAPSHOT_CHANGED"
    );
    assert!(status_for(&state, &project.id).await.unwrap().can_undo);
    assert_eq!(fixture::logical(&state, &project.id).await, before);
}

#[tokio::test]
async fn closed_reader_is_invalidated_by_open_and_by_explicit_close_without_open() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let project = fixture::project(&state, 16).await;
    let snapshot = repository::capture_history_state(&state, &project.id)
        .await
        .unwrap();
    let files = fixture::files(temp.path());
    state.project_access.release(&project.id).unwrap();
    assert_eq!(
        from_snapshot(&state, &snapshot)
            .await
            .err()
            .unwrap()
            .to_string(),
        "PROJECT_SNAPSHOT_CHANGED"
    );
    assert_eq!(fixture::files(temp.path()), files);
    let snapshot = repository::capture_history_state(&state, &project.id)
        .await
        .unwrap();
    state
        .project_access
        .acquire(&state.root, &project.id)
        .unwrap();
    assert_eq!(
        from_snapshot(&state, &snapshot)
            .await
            .err()
            .unwrap()
            .to_string(),
        "PROJECT_SNAPSHOT_CHANGED"
    );
}

#[tokio::test]
async fn repeated_writer_acquire_and_other_project_lifecycle_do_not_stale_history() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let project = fixture::project(&state, 16).await;
    state
        .project_access
        .acquire(&state.root, &project.id)
        .unwrap();
    let snapshot = repository::capture_history_state(&state, &project.id)
        .await
        .unwrap();
    state
        .project_access
        .acquire(&state.root, &project.id)
        .unwrap();
    let other = uuid::Uuid::new_v4().to_string();
    state.project_access.acquire(&state.root, &other).unwrap();
    state.project_access.release(&other).unwrap();
    assert!(from_snapshot(&state, &snapshot).await.unwrap().can_undo);
}

#[tokio::test]
async fn read_only_history_does_not_upgrade_access_and_explicit_reacquire_invalidates_it() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let project = fixture::project(&state, 16).await;
    let files = fixture::files(temp.path());
    let owner = AccessRegistry::default();
    owner.acquire(&state.root, &project.id).unwrap();
    let owner_path = format!(
        ".project-access{}{}.owner.json",
        std::path::MAIN_SEPARATOR,
        project.id
    );
    let lock_path = format!(
        ".project-access{}{}.lock",
        std::path::MAIN_SEPARATOR,
        project.id
    );
    let owner_bytes = std::fs::read(state.root.join(&owner_path)).unwrap();
    assert_eq!(
        state
            .project_access
            .acquire(&state.root, &project.id)
            .unwrap()
            .mode,
        "read_only"
    );
    let snapshot = repository::capture_history_state(&state, &project.id)
        .await
        .unwrap();
    let before = fixture::logical(&state, &project.id).await;
    assert!(from_snapshot(&state, &snapshot).await.unwrap().can_undo);
    owner.release(&project.id).unwrap();
    assert!(from_snapshot(&state, &snapshot).await.unwrap().can_undo);
    assert_eq!(
        state
            .project_access
            .status(&state.root, &project.id)
            .unwrap()
            .mode,
        "read_only"
    );
    assert!(state
        .project_access
        .ensure_write(&state.root, &project.id)
        .is_err());
    assert_eq!(fixture::logical(&state, &project.id).await, before);
    // Windows cannot read the exclusively locked ranges. Inspect after the owner
    // releases; both informational and lock bytes must still match its original record.
    let mut after_files = fixture::files(temp.path());
    assert_eq!(after_files.remove(&owner_path).unwrap(), owner_bytes);
    assert_eq!(after_files.remove(&lock_path).unwrap(), owner_bytes);
    assert_eq!(after_files, files);
    assert_eq!(
        state
            .project_access
            .acquire(&state.root, &project.id)
            .unwrap()
            .mode,
        "write"
    );
    assert_eq!(
        from_snapshot(&state, &snapshot)
            .await
            .err()
            .unwrap()
            .to_string(),
        "PROJECT_SNAPSHOT_CHANGED"
    );
}

#[tokio::test]
async fn changing_location_or_registry_rejects_history_without_recovery() {
    let temp = tempfile::tempdir().unwrap();
    let other_root = tempfile::tempdir().unwrap();
    let mut state = fixture::state(temp.path()).await;
    let project = fixture::project(&state, 16).await;
    let snapshot = repository::capture_history_state(&state, &project.id)
        .await
        .unwrap();
    let before = fixture::logical(&state, &project.id).await;
    state.root = other_root.path().into();
    assert_eq!(
        from_snapshot(&state, &snapshot)
            .await
            .err()
            .unwrap()
            .to_string(),
        "PROJECT_SNAPSHOT_CHANGED"
    );
    assert!(fixture::files(other_root.path()).is_empty());
    state.root = temp.path().into();
    state.project_access = AccessRegistry::default();
    assert_eq!(
        from_snapshot(&state, &snapshot)
            .await
            .err()
            .unwrap()
            .to_string(),
        "PROJECT_SNAPSHOT_CHANGED"
    );
    assert_eq!(fixture::logical(&state, &project.id).await, before);
}

#[tokio::test]
async fn new_pending_recovery_invalidates_history_without_publishing_it() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let project = fixture::project(&state, 16).await;
    let snapshot = repository::capture_history_state(&state, &project.id)
        .await
        .unwrap();
    fixture::pending(&state, &project.id).await;
    let before = fixture::logical(&state, &project.id).await;
    let files = fixture::files(temp.path());
    assert_eq!(
        from_snapshot(&state, &snapshot)
            .await
            .err()
            .unwrap()
            .to_string(),
        "PROJECT_SNAPSHOT_RECOVERY_REQUIRED"
    );
    assert_eq!(fixture::logical(&state, &project.id).await, before);
    assert_eq!(fixture::files(temp.path()), files);
}
