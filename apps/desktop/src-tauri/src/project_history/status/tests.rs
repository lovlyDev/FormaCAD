use super::*;
use crate::project_repository::legacy;
use crate::project_repository::legacy::read_test_fixture as fixture;

#[tokio::test]
async fn history_controls_are_coherent_and_do_not_wait_for_global_write_lock() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let project = fixture::project(&state, 16).await;
    let before = fixture::logical(&state, &project.id).await;
    let files = fixture::files(temp.path());
    let _held = state.writes.lock().await;
    let status = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        status_for(&state, &project.id),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(status.can_undo);
    assert!(!status.can_redo);
    assert_eq!(status.expected_revision, project.current_revision);
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
async fn pending_history_status_does_not_recover_or_create_a_folder() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let project = fixture::project(&state, 16).await;
    fixture::pending(&state, &project.id).await;
    let before = fixture::logical(&state, &project.id).await;
    let files = fixture::files(temp.path());
    assert_eq!(
        status_for(&state, &project.id)
            .await
            .err()
            .unwrap()
            .to_string(),
        "PROJECT_SNAPSHOT_RECOVERY_REQUIRED"
    );
    assert_eq!(fixture::logical(&state, &project.id).await, before);
    assert_eq!(fixture::files(temp.path()), files);
    assert!(!state.root.join(&project.id).exists());
}

#[tokio::test]
async fn captured_history_cannot_be_returned_after_committed_head_changes() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let project = fixture::project(&state, 16).await;
    let snapshot = legacy::capture_history_state(&state, &project.id)
        .await
        .unwrap();
    let latest = fixture::next_head(&state, &project).await;
    let before = fixture::logical(&state, &project.id).await;
    assert_eq!(
        from_snapshot(&state, &snapshot)
            .await
            .err()
            .unwrap()
            .to_string(),
        "PROJECT_SNAPSHOT_CHANGED"
    );
    assert_eq!(fixture::logical(&state, &project.id).await, before);
    assert_eq!(
        status_for(&state, &project.id)
            .await
            .unwrap()
            .expected_revision,
        latest.current_revision
    );
    assert!(!state.root.join(&project.id).exists());
}

#[tokio::test]
async fn large_sql_only_embedded_legacy_history_is_readable_without_materialization() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let project = fixture::project(&state, 24 * 1024 * 1024 + 1).await;
    let status = status_for(&state, &project.id).await.unwrap();
    assert_eq!(status.expected_revision, project.current_revision);
    assert!(status.can_undo);
    assert!(!state.root.join(&project.id).exists());
}

#[tokio::test]
async fn archive_event_limit_does_not_reject_a_valid_committed_cursor() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let project = fixture::project(&state, 16).await;
    // Real SQL audit rows exceed the portable export budget. The separately
    // committed typed cursor remains valid; this is not 30,001 CAD operations.
    sqlx::query("WITH RECURSIVE sequence(n) AS (SELECT 1 UNION ALL SELECT n+1 FROM sequence WHERE n<30001) INSERT INTO project_history_events(project_id,kind,from_revision,to_revision,target_revision,created_at) SELECT ?,'edit',NULL,?,?,? FROM sequence")
        .bind(&project.id).bind(&project.current_revision).bind(&project.current_revision)
        .bind("2026-10-09T00:00:00Z").execute(&state.pool).await.unwrap();
    let before = fixture::logical(&state, &project.id).await;
    assert!(before.3 > 30000);
    let status = status_for(&state, &project.id).await.unwrap();
    assert!(status.can_undo);
    assert_eq!(status.expected_revision, project.current_revision);
    assert_eq!(
        legacy::capture_with_history(&state, &project.id)
            .await
            .err()
            .unwrap()
            .to_string(),
        "HISTORY_ARCHIVE_TOO_LARGE"
    );
    assert_eq!(fixture::logical(&state, &project.id).await, before);
    assert!(!state.root.join(&project.id).exists());
}

#[tokio::test]
async fn cursor_generation_and_public_snapshot_mutation_are_detected_without_head_change() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let project = fixture::project(&state, 16).await;
    let snapshot = legacy::capture_history_state(&state, &project.id)
        .await
        .unwrap();
    let mut raw: serde_json::Value =
        serde_json::from_str(&fixture::logical(&state, &project.id).await.1).unwrap();
    raw["generation"] = serde_json::json!(1);
    sqlx::query("UPDATE project_history SET state_json=? WHERE project_id=?")
        .bind(raw.to_string())
        .bind(&project.id)
        .execute(&state.pool)
        .await
        .unwrap();
    assert_eq!(
        from_snapshot(&state, &snapshot)
            .await
            .err()
            .unwrap()
            .to_string(),
        "PROJECT_SNAPSHOT_CHANGED"
    );
    let mut latest = legacy::capture_history_state(&state, &project.id)
        .await
        .unwrap();
    latest.history_state = crate::project_history::HistoryState::from_project(&project).unwrap();
    assert_eq!(
        legacy::recheck_history_state(&state, &latest)
            .await
            .err()
            .unwrap()
            .to_string(),
        "PROJECT_SNAPSHOT_CHANGED"
    );
}

#[tokio::test]
async fn oversized_cursor_state_is_bounded_before_decoding() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let project = fixture::project(&state, 16).await;
    sqlx::query("UPDATE project_history SET state_json=? WHERE project_id=?")
        .bind(" ".repeat(16 * 1024 * 1024 + 1))
        .bind(&project.id)
        .execute(&state.pool)
        .await
        .unwrap();
    assert_eq!(
        status_for(&state, &project.id)
            .await
            .err()
            .unwrap()
            .to_string(),
        "HISTORY_ARCHIVE_TOO_LARGE"
    );
    assert!(!state.root.join(&project.id).exists());
}

#[tokio::test]
async fn cursor_recheck_rejects_changed_managed_cas_and_preserves_corrupt_bytes() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let mut project = fixture::project(&state, 16).await;
    crate::artifacts::normalize(&mut project).unwrap();
    fixture::replace(&state, &project, false).await;
    let folder = state.root.join(&project.id);
    // stage_with expects an existing checked root, as its production caller supplies.
    std::fs::create_dir(&folder).unwrap();
    let staged =
        crate::project_storage_v2::stage_with(&folder, &project, |_| Ok(vec![b'x'; 16])).unwrap();
    crate::project_storage_v2::publish(&folder, staged).unwrap();
    let snapshot = legacy::capture_history_state(&state, &project.id)
        .await
        .unwrap();
    let asset = folder
        .join("assets")
        .join(project.files[0].sha256.as_ref().unwrap());
    std::fs::write(&asset, vec![b'y'; 16]).unwrap();
    let before = fixture::files(temp.path());
    assert!(legacy::recheck_history_state(&state, &snapshot)
        .await
        .is_err());
    assert_eq!(fixture::files(temp.path()), before);
    assert_eq!(std::fs::read(asset).unwrap(), vec![b'y'; 16]);
}
