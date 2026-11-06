//! Lifecycle fencing for full project snapshots, independent of history status.
use super::{
    capture_project, capture_with_history, read_test_fixture as fixture, recheck, recheck_lifecycle,
};
use std::{future::Future, task::Poll};

#[tokio::test]
async fn committed_metadata_changes_preserve_lifecycle_but_not_the_old_saved_snapshot() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let mut project = fixture::project(&state, 16).await;
    let pending = crate::artifacts::normalize(&mut project).unwrap();
    // Materialize the original managed source BEFORE capturing its location:
    // a missing managed directory becoming present is a different lifecycle.
    let baseline = crate::projects::persist(&state, project, pending, None)
        .await
        .unwrap();
    let captured = capture_with_history(&state, &baseline.id).await.unwrap();
    let original_history = captured.history.clone();
    let mut metadata = baseline.clone();
    metadata.name = "Committed metadata winner".into();
    let committed = crate::projects::persist(&state, metadata, Vec::new(), None)
        .await
        .unwrap();
    let before = fixture::logical(&state, &baseline.id).await;
    let files = fixture::files(temp.path());
    assert_eq!(committed.current_revision, baseline.current_revision);
    assert_eq!(committed.revisions, baseline.revisions);
    recheck_lifecycle(&state, &captured).unwrap();
    assert_eq!(
        recheck(&state, &captured).await.err().unwrap().to_string(),
        "PROJECT_SNAPSHOT_CHANGED"
    );
    let fresh = capture_with_history(&state, &baseline.id).await.unwrap();
    assert_eq!(fresh.history, original_history);
    recheck(&state, &fresh).await.unwrap();
    assert_eq!(fixture::logical(&state, &baseline.id).await, before);
    assert_eq!(fixture::files(temp.path()), files);

    state.project_access.release(&baseline.id).unwrap();
    state
        .project_access
        .acquire(&state.root, &baseline.id)
        .unwrap();
    for snapshot in [&captured, &fresh] {
        assert_eq!(
            recheck_lifecycle(&state, snapshot)
                .err()
                .unwrap()
                .to_string(),
            "PROJECT_SNAPSHOT_CHANGED"
        );
    }
    assert_eq!(fixture::logical(&state, &baseline.id).await, before);
    state.project_access.release(&baseline.id).unwrap();
}

#[tokio::test]
async fn lifecycle_recheck_rejects_caller_mutation_of_captured_project_or_archive() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let project = fixture::project(&state, 16).await;
    let mut plain = capture_project(&state, &project.id).await.unwrap();
    let mut archive = capture_with_history(&state, &project.id).await.unwrap();
    let before = fixture::logical(&state, &project.id).await;
    let files = fixture::files(temp.path());
    plain.project.name.push('!');
    archive.history.as_mut().unwrap().push(b' ');
    for snapshot in [&plain, &archive] {
        assert_eq!(
            recheck_lifecycle(&state, snapshot)
                .err()
                .unwrap()
                .to_string(),
            "PROJECT_SNAPSHOT_CHANGED"
        );
    }
    assert_eq!(fixture::logical(&state, &project.id).await, before);
    assert_eq!(fixture::files(temp.path()), files);
}

#[tokio::test]
async fn replaced_directory_at_the_same_path_rejects_snapshot_with_identical_sql() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let project = fixture::project(&state, 16).await;
    let path = state.root.join(&project.id);
    std::fs::create_dir(&path).unwrap();
    let snapshot = capture_project(&state, &project.id).await.unwrap();
    let before = fixture::logical(&state, &project.id).await;
    std::fs::rename(&path, state.root.join("old-source")).unwrap();
    std::fs::create_dir(&path).unwrap();
    let files = fixture::files(temp.path());
    assert_eq!(
        recheck(&state, &snapshot).await.err().unwrap().to_string(),
        "PROJECT_SNAPSHOT_CHANGED"
    );
    assert_eq!(fixture::logical(&state, &project.id).await, before);
    assert_eq!(fixture::files(temp.path()), files);
    recheck(&state, &capture_project(&state, &project.id).await.unwrap())
        .await
        .unwrap();
}

#[tokio::test]
async fn equal_project_ids_and_payloads_at_different_roots_do_not_exchange_snapshots() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let state = fixture::state(first.path()).await;
    let other = fixture::state(second.path()).await;
    let project = fixture::project(&state, 16).await;
    fixture::replace(&other, &project, true).await;
    let snapshot = capture_project(&state, &project.id).await.unwrap();
    let before = fixture::logical(&other, &project.id).await;
    assert_eq!(
        recheck(&other, &snapshot).await.err().unwrap().to_string(),
        "PROJECT_SNAPSHOT_CHANGED"
    );
    assert_eq!(fixture::logical(&other, &project.id).await, before);
}

#[tokio::test]
async fn close_and_reopen_reject_both_project_and_archive_snapshots_without_writes() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let project = fixture::project(&state, 16).await;
    state
        .project_access
        .acquire(&state.root, &project.id)
        .unwrap();
    let plain = capture_project(&state, &project.id).await.unwrap();
    let archive = capture_with_history(&state, &project.id).await.unwrap();
    let before = fixture::logical(&state, &project.id).await;
    state.project_access.release(&project.id).unwrap();
    state
        .project_access
        .acquire(&state.root, &project.id)
        .unwrap();
    for snapshot in [&plain, &archive] {
        assert_eq!(
            recheck(&state, snapshot).await.err().unwrap().to_string(),
            "PROJECT_SNAPSHOT_CHANGED"
        );
    }
    let fresh = capture_with_history(&state, &project.id).await.unwrap();
    recheck(&state, &fresh).await.unwrap();
    assert_eq!(fixture::logical(&state, &project.id).await, before);
}

#[tokio::test]
async fn close_during_project_capture_sql_wait_rejects_unchanged_source() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let project = fixture::project(&state, 16).await;
    let before = fixture::logical(&state, &project.id).await;
    let files = fixture::files(temp.path());
    let mut held = Vec::new();
    for _ in 0..state.pool.options().get_max_connections() {
        held.push(state.pool.acquire().await.unwrap());
    }
    let capture = capture_project(&state, &project.id);
    tokio::pin!(capture);
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
async fn close_during_full_recheck_sql_wait_does_not_refresh_original_session() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let project = fixture::project(&state, 16).await;
    let snapshot = capture_with_history(&state, &project.id).await.unwrap();
    let before = fixture::logical(&state, &project.id).await;
    let files = fixture::files(temp.path());
    let mut held = Vec::new();
    for _ in 0..state.pool.options().get_max_connections() {
        held.push(state.pool.acquire().await.unwrap());
    }
    let checking = recheck(&state, &snapshot);
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
