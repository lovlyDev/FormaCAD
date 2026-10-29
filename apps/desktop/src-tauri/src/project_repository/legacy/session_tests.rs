//! Lifecycle fencing for full project snapshots, independent of history status.
use super::{capture_project, capture_with_history, read_test_fixture as fixture, recheck};
use std::{future::Future, task::Poll};

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
