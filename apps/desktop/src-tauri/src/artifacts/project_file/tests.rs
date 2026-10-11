use super::*;
use crate::project_repository::legacy::read_test_fixture as fixture;

#[tokio::test]
async fn attachment_read_is_pure_and_does_not_wait_for_write_lock() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let project = fixture::project(&state, 16).await;
    let before = fixture::logical(&state, &project.id).await;
    let files = fixture::files(temp.path());
    let _held = state.writes.lock().await;
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        read_for(&state, &project.id, "model.step"),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(
        crate::artifacts::decode_data(&result).unwrap(),
        vec![b'x'; 16]
    );
    assert_eq!(fixture::logical(&state, &project.id).await, before);
    assert_eq!(fixture::files(temp.path()), files);
    assert_eq!(
        state
            .project_access
            .status(&state.root, &project.id)
            .unwrap()
            .mode,
        "closed"
    );
}

#[tokio::test]
async fn pending_attachment_read_refuses_recovery_without_files_or_journal_cleanup() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let project = fixture::project(&state, 16).await;
    fixture::pending(&state, &project.id).await;
    let before = fixture::logical(&state, &project.id).await;
    let files = fixture::files(temp.path());
    assert_eq!(
        read_for(&state, &project.id, "model.step")
            .await
            .err()
            .unwrap()
            .to_string(),
        "PROJECT_SNAPSHOT_RECOVERY_REQUIRED"
    );
    assert_eq!(fixture::logical(&state, &project.id).await, before);
    assert_eq!(fixture::files(temp.path()), files);
}

#[tokio::test]
async fn old_attachment_capture_is_rejected_after_sql_head_changes() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let project = fixture::project(&state, 16).await;
    let snapshot = legacy::capture_project(&state, &project.id).await.unwrap();
    fixture::next_head(&state, &project).await;
    let before = fixture::logical(&state, &project.id).await;
    assert_eq!(
        from_snapshot(&state, &snapshot, "model.step")
            .await
            .err()
            .unwrap()
            .to_string(),
        "PROJECT_SNAPSHOT_CHANGED"
    );
    assert_eq!(fixture::logical(&state, &project.id).await, before);
    assert!(!state.root.join(&project.id).exists());
}

#[tokio::test]
async fn large_embedded_legacy_attachment_returns_exact_bytes_without_materialization() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let project = fixture::project(&state, 24 * 1024 * 1024 + 1).await;
    let result = read_for(&state, &project.id, "model.step").await.unwrap();
    assert_eq!(
        crate::artifacts::decode_data(&result).unwrap(),
        vec![b'x'; 24 * 1024 * 1024 + 1]
    );
    assert!(!state.root.join(&project.id).exists());
}

#[tokio::test]
async fn inline_size_and_present_hash_corruption_fail_without_writes() {
    for size_corrupt in [true, false] {
        let temp = tempfile::tempdir().unwrap();
        let state = fixture::state(temp.path()).await;
        let mut project = fixture::project(&state, 16).await;
        if size_corrupt {
            project.files[0].size += 1;
        } else {
            project.files[0].sha256 = Some("a".repeat(64));
        }
        fixture::replace(&state, &project, false).await;
        let before = fixture::logical(&state, &project.id).await;
        assert_eq!(
            read_for(&state, &project.id, "model.step")
                .await
                .err()
                .unwrap()
                .to_string(),
            "Project file failed its integrity check"
        );
        assert_eq!(fixture::logical(&state, &project.id).await, before);
        assert!(!state.root.join(&project.id).exists());
    }
}
