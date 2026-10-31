//! Actual controls router: metadata authority is distinct from deep CAS health.
use super::*;
use crate::{
    models::{Message, Project},
    project_repository::legacy::{self, read_test_fixture as fixture},
};

async fn normalized(state: &AppState) -> Project {
    let mut project = fixture::project(state, 1024 * 1024).await;
    crate::artifacts::normalize(&mut project).unwrap();
    project.messages.push(Message {
        id: "large-chat".into(),
        role: "assistant".into(),
        text: "chat \\\" unicode 世界 ".repeat(120_000),
        created_at: project.created_at.clone(),
    });
    fixture::replace(state, &project, false).await;
    project
}

async fn mirror(state: &AppState, project: &Project) -> std::path::PathBuf {
    let root = state.root.join(&project.id);
    std::fs::create_dir(&root).unwrap();
    let mut staged =
        crate::project_storage_v2::stage_with(&root, project, |_| Ok(vec![b'x'; 1024 * 1024]))
            .unwrap();
    let history =
        crate::project_history::export_history(&mut state.pool.acquire().await.unwrap(), project)
            .await
            .unwrap();
    crate::project_storage_v2::stage_history(&root, &mut staged, &history).unwrap();
    crate::project_storage_v2::publish(&root, staged).unwrap();
    root
}

#[tokio::test]
async fn actual_fast_controls_skip_unrelated_cas_and_archive_but_reject_metadata_corruption() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let project = normalized(&state).await;
    let root = mirror(&state, &project).await;
    let manifest: crate::project_storage_v2::Manifest =
        serde_json::from_slice(&std::fs::read(root.join("manifest.json")).unwrap()).unwrap();
    let asset = root
        .join("assets")
        .join(project.files[0].sha256.as_ref().unwrap());
    std::fs::remove_file(&asset).unwrap();
    std::fs::create_dir(&asset).unwrap();
    let history = root
        .join("metadata/generations")
        .join(&manifest.generation)
        .join("history.json");
    std::fs::remove_file(&history).unwrap();
    std::fs::create_dir(&history).unwrap();
    let before = fixture::files(temp.path());
    let snapshot = legacy::capture_controls(&state, &project.id).await.unwrap();
    assert!(snapshot.fallback().is_none());
    let status = status_for(&state, &project.id).await.unwrap();
    assert!(status.can_undo);
    assert!(!status.can_redo);
    assert_eq!(status.expected_revision, project.current_revision);
    assert!(legacy::capture_history_state(&state, &project.id)
        .await
        .is_err());
    assert_eq!(fixture::files(temp.path()), before);
    let metadata = root
        .join("metadata/generations")
        .join(&manifest.generation)
        .join("project.json");
    let mut bytes = std::fs::read(&metadata).unwrap();
    let last = bytes.len() - 1;
    bytes[last] = b' ';
    std::fs::write(&metadata, &bytes).unwrap();
    assert!(status_for(&state, &project.id).await.is_err());
    assert_eq!(std::fs::read(metadata).unwrap(), bytes);
    assert!(asset.is_dir() && history.is_dir());
}

#[tokio::test]
async fn fast_original_seals_reject_ignored_chat_and_raw_cursor_drift() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let mut project = normalized(&state).await;
    let snapshot = legacy::capture_controls(&state, &project.id).await.unwrap();
    assert!(snapshot.fallback().is_none());
    project.messages[0].text.push('!');
    fixture::replace(&state, &project, false).await;
    assert_eq!(
        snapshot.recheck(&state).await.err().unwrap().to_string(),
        "PROJECT_SNAPSHOT_CHANGED"
    );
    let snapshot = legacy::capture_controls(&state, &project.id).await.unwrap();
    sqlx::query("UPDATE project_history SET state_json=state_json || ' ' WHERE project_id=?")
        .bind(&project.id)
        .execute(&state.pool)
        .await
        .unwrap();
    assert_eq!(
        snapshot.recheck(&state).await.err().unwrap().to_string(),
        "PROJECT_SNAPSHOT_CHANGED"
    );
    assert!(status_for(&state, &project.id).await.unwrap().can_undo);
}

#[tokio::test]
async fn fast_original_session_rejects_other_root_and_close_during_sql_wait() {
    use std::{future::Future, task::Poll};
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let project = normalized(&state).await;
    let snapshot = legacy::capture_controls(&state, &project.id).await.unwrap();
    let other = tempfile::tempdir().unwrap();
    let other_state = fixture::state(other.path()).await;
    fixture::replace(&other_state, &project, true).await;
    assert_eq!(
        snapshot
            .recheck(&other_state)
            .await
            .err()
            .unwrap()
            .to_string(),
        "PROJECT_SNAPSHOT_CHANGED"
    );
    let mut held = Vec::new();
    for _ in 0..state.pool.options().get_max_connections() {
        held.push(state.pool.acquire().await.unwrap());
    }
    let check = snapshot.recheck(&state);
    tokio::pin!(check);
    std::future::poll_fn(|cx| {
        assert!(check.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    state.project_access.release(&project.id).unwrap();
    drop(held);
    assert_eq!(
        check.await.err().unwrap().to_string(),
        "PROJECT_SNAPSHOT_CHANGED"
    );
}

#[tokio::test]
async fn inline_and_noncanonical_mirror_take_the_existing_full_fallback() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let inline = fixture::project(&state, 16).await;
    assert!(legacy::capture_controls(&state, &inline.id)
        .await
        .unwrap()
        .fallback()
        .is_some());
    assert!(status_for(&state, &inline.id).await.unwrap().can_undo);
    let project = normalized(&state).await;
    let root = mirror(&state, &project).await;
    let mut manifest: crate::project_storage_v2::Manifest =
        serde_json::from_slice(&std::fs::read(root.join("manifest.json")).unwrap()).unwrap();
    let bytes = serde_json::to_vec_pretty(&project).unwrap();
    std::fs::write(
        root.join("metadata/generations")
            .join(&manifest.generation)
            .join("project.json"),
        &bytes,
    )
    .unwrap();
    let entry = manifest
        .files
        .iter_mut()
        .find(|entry| entry.path == "project.json")
        .unwrap();
    entry.size = bytes.len() as u64;
    entry.sha256 = crate::artifacts::digest(&bytes);
    std::fs::write(
        root.join("manifest.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    assert!(legacy::capture_controls(&state, &project.id)
        .await
        .unwrap()
        .fallback()
        .is_some());
    assert!(status_for(&state, &project.id).await.unwrap().can_undo);
}

#[tokio::test]
async fn fast_controls_preserve_materialized_undo_redo_and_reject_changed_model_head() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let project = normalized(&state).await;
    let mut current = fixture::next_head(&state, &project).await;
    for direction in [
        crate::project_history::Direction::Undo,
        crate::project_history::Direction::Redo,
    ] {
        let history = crate::project_history::load_state(&state.pool, &current)
            .await
            .unwrap();
        let (next, prepared) = crate::project_history::prepare(
            &current,
            &history,
            direction,
            current.current_revision.as_deref(),
            &uuid::Uuid::new_v4().to_string(),
            "2026-10-31T00:00:00Z",
        )
        .unwrap();
        let mut transaction = state.pool.begin().await.unwrap();
        sqlx::query("UPDATE projects SET payload=? WHERE id=?")
            .bind(serde_json::to_string(&next).unwrap())
            .bind(&next.id)
            .execute(&mut *transaction)
            .await
            .unwrap();
        crate::project_history::record_commit(
            &mut transaction,
            Some(&current),
            &next,
            Some(&prepared),
        )
        .await
        .unwrap();
        transaction.commit().await.unwrap();
        let fast = legacy::capture_controls(&state, &next.id).await.unwrap();
        assert!(fast.fallback().is_none());
        let full = legacy::capture_history_state(&state, &next.id)
            .await
            .unwrap();
        let old_status = from_snapshot(&state, &full).await.unwrap();
        let status = status_for(&state, &next.id).await.unwrap();
        assert_eq!(status.can_undo, old_status.can_undo);
        assert_eq!(status.can_redo, old_status.can_redo);
        assert_eq!(status.expected_revision, old_status.expected_revision);
        current = next;
    }
    // The final materialized redo head must equal its logical baseline model.
    current.revisions.last_mut().unwrap().parameters.width += 1.0;
    fixture::replace(&state, &current, false).await;
    assert_eq!(
        status_for(&state, &current.id)
            .await
            .err()
            .unwrap()
            .to_string(),
        "HISTORY_INVALID_STATE"
    );
}

#[tokio::test]
async fn fast_controls_reject_manifest_file_reference_mismatch_without_repair() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let project = normalized(&state).await;
    let root = mirror(&state, &project).await;
    let manifest_path = root.join("manifest.json");
    let mut manifest: crate::project_storage_v2::Manifest =
        serde_json::from_slice(&std::fs::read(&manifest_path).unwrap()).unwrap();
    manifest
        .files
        .retain(|entry| !entry.path.starts_with("attachments/"));
    let bytes = serde_json::to_vec(&manifest).unwrap();
    std::fs::write(&manifest_path, &bytes).unwrap();
    let before = fixture::files(temp.path());
    assert_eq!(
        status_for(&state, &project.id)
            .await
            .err()
            .unwrap()
            .to_string(),
        "PROJECT_SNAPSHOT_CHANGED"
    );
    assert_eq!(std::fs::read(manifest_path).unwrap(), bytes);
    assert_eq!(fixture::files(temp.path()), before);
}
