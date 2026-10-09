#[path = "project_repository/fixture.rs"]
mod fixture;

use forma_core::{project_access::copy::copy_for_editing, project_repository::legacy};
use serde_json::{json, Value};

#[tokio::test]
async fn coherent_project_and_history_capture_is_entirely_read_only() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let project = fixture::project(&state).await;
    let before = fixture::snapshot(&state, &project.id).await;
    assert_eq!(
        state
            .project_access
            .status(&state.root, &project.id)
            .unwrap()
            .mode,
        "closed"
    );
    let model_only = legacy::capture_project(&state, &project.id).await.unwrap();
    assert!(model_only.history.is_none());
    let captured = legacy::capture_with_history(&state, &project.id)
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(&captured.project).unwrap(),
        serde_json::to_value(&project).unwrap()
    );
    let archive = captured.history.as_ref().unwrap();
    assert_eq!(archive, &fixture::history(&state, &project).await);
    forma_core::project_history::validate_archive(&captured.project, archive).unwrap();
    let archive: Value = serde_json::from_slice(archive).unwrap();
    assert_eq!(archive["state"]["head"], json!(project.current_revision));
    assert_eq!(archive["events"].as_array().unwrap().len(), 2);
    legacy::recheck(&state, &captured).await.unwrap();
    assert_eq!(
        state
            .project_access
            .status(&state.root, &project.id)
            .unwrap()
            .mode,
        "closed"
    );
    assert_eq!(fixture::snapshot(&state, &project.id).await, before);
}

#[tokio::test]
async fn pending_recovery_is_rejected_by_capture_and_copy_without_publication_or_cleanup() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let project = fixture::project(&state).await;
    fixture::pending(&state, &project).await;
    let before = fixture::snapshot(&state, &project.id).await;
    for include_history in [false, true] {
        let result = if include_history {
            legacy::capture_with_history(&state, &project.id).await
        } else {
            legacy::capture_project(&state, &project.id).await
        };
        assert_eq!(
            result.err().unwrap().to_string(),
            "PROJECT_SNAPSHOT_RECOVERY_REQUIRED"
        );
        assert_eq!(fixture::snapshot(&state, &project.id).await, before);
    }
    assert_eq!(
        copy_for_editing(&state, &project.id)
            .await
            .err()
            .unwrap()
            .to_string(),
        "PROJECT_SNAPSHOT_RECOVERY_REQUIRED"
    );
    assert_eq!(fixture::snapshot(&state, &project.id).await, before);
}

#[tokio::test]
async fn malformed_manifest_or_corrupt_cas_is_not_repaired_by_snapshot_read() {
    for corruption in ["manifest", "asset", "metadata"] {
        let temp = tempfile::tempdir().unwrap();
        let state = fixture::state(temp.path()).await;
        let project = fixture::project(&state).await;
        let root = state.root.join(&project.id);
        let manifest: Value =
            serde_json::from_slice(&std::fs::read(root.join("manifest.json")).unwrap()).unwrap();
        let path = match corruption {
            "manifest" => root.join("manifest.json"),
            "asset" => root
                .join("assets")
                .join(project.files[0].sha256.as_ref().unwrap()),
            _ => root
                .join("metadata/generations")
                .join(manifest["generation"].as_str().unwrap())
                .join("project.json"),
        };
        std::fs::write(path, b"corrupt test fixture").unwrap();
        let before = fixture::snapshot(&state, &project.id).await;
        assert!(
            legacy::capture_with_history(&state, &project.id)
                .await
                .is_err(),
            "{corruption}"
        );
        assert_eq!(fixture::snapshot(&state, &project.id).await, before);
    }
}

#[tokio::test]
async fn sql_payload_foreign_identity_is_rejected_without_repairing_the_index() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let project = fixture::project(&state).await;
    let mut foreign = project.clone();
    foreign.id = uuid::Uuid::new_v4().to_string();
    sqlx::query("UPDATE projects SET payload=? WHERE id=?")
        .bind(serde_json::to_string(&foreign).unwrap())
        .bind(&project.id)
        .execute(&state.pool)
        .await
        .unwrap();
    let before = fixture::snapshot(&state, &project.id).await;
    assert_eq!(
        legacy::capture_with_history(&state, &project.id)
            .await
            .err()
            .unwrap()
            .to_string(),
        "PROJECT_SNAPSHOT_CHANGED"
    );
    assert_eq!(fixture::snapshot(&state, &project.id).await, before);
}

#[tokio::test]
async fn actual_editable_copy_preserves_source_bytes_and_history_with_new_project_uuid() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let project = fixture::project(&state).await;
    let source_root = state.root.join(&project.id);
    let source_files = fixture::files(&source_root);
    let before = fixture::snapshot(&state, &project.id).await;
    let history = fixture::history(&state, &project).await;
    let copy = copy_for_editing(&state, &project.id).await.unwrap();
    assert_ne!(copy.id, project.id);
    assert_eq!(copy.revisions, project.revisions);
    assert_eq!(copy.current_revision, project.current_revision);
    assert_eq!(fixture::history(&state, &copy).await, history);
    assert_eq!(fixture::files(&source_root), source_files);
    let after = fixture::snapshot(&state, &project.id).await;
    assert_eq!(after.payload, before.payload);
    assert_eq!(after.history, before.history);
    assert_eq!(after.events, before.events);
    assert_eq!(after.journal, before.journal);
    let reopened = forma_core::project_storage_v2::read_folder(&state.root.join(&copy.id)).unwrap();
    assert_eq!(
        serde_json::to_value(reopened).unwrap(),
        serde_json::to_value(&copy).unwrap()
    );
    let captured = legacy::capture_with_history(&state, &copy.id)
        .await
        .unwrap();
    assert_eq!(captured.history.as_ref().unwrap(), &history);
    for file in &copy.files {
        let bytes = forma_core::artifacts::read(&state, &copy.id, file).unwrap();
        assert_eq!(
            forma_core::artifacts::digest(&bytes),
            *file.sha256.as_ref().unwrap()
        );
    }
}

#[tokio::test]
async fn uncommitted_metadata_winner_does_not_mix_snapshot_and_final_recheck_rejects_commit() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let project = fixture::project(&state).await;
    let mut winner = project.clone();
    winner.name = "Concurrent metadata winner".into();
    let mut writer = state.pool.begin().await.unwrap();
    sqlx::query("UPDATE projects SET name=?,payload=? WHERE id=?")
        .bind(&winner.name)
        .bind(serde_json::to_string(&winner).unwrap())
        .bind(&project.id)
        .execute(&mut *writer)
        .await
        .unwrap();
    // WAL reader must observe the old committed project/history, never the pending payload.
    let captured = legacy::capture_with_history(&state, &project.id)
        .await
        .unwrap();
    assert_eq!(captured.project.name, project.name);
    assert_eq!(
        captured.history.as_ref().unwrap(),
        &fixture::history(&state, &project).await
    );
    writer.commit().await.unwrap();
    let before_recheck = fixture::snapshot(&state, &project.id).await;
    assert_eq!(
        legacy::recheck(&state, &captured)
            .await
            .err()
            .unwrap()
            .to_string(),
        "PROJECT_SNAPSHOT_CHANGED"
    );
    assert_eq!(fixture::snapshot(&state, &project.id).await, before_recheck);
}

#[tokio::test]
async fn captured_history_is_bound_exactly_even_when_project_head_is_unchanged() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let project = fixture::project(&state).await;
    let captured = legacy::capture_with_history(&state, &project.id)
        .await
        .unwrap();
    sqlx::query("UPDATE project_history_events SET created_at=? WHERE project_id=?")
        .bind("2026-10-08T12:00:00Z")
        .bind(&project.id)
        .execute(&state.pool)
        .await
        .unwrap();
    let before = fixture::snapshot(&state, &project.id).await;
    assert_eq!(
        legacy::recheck(&state, &captured)
            .await
            .err()
            .unwrap()
            .to_string(),
        "PROJECT_SNAPSHOT_CHANGED"
    );
    assert_eq!(fixture::snapshot(&state, &project.id).await, before);
}

#[tokio::test]
async fn missing_project_is_explicit_and_does_not_create_project_folder_or_lease() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let id = uuid::Uuid::new_v4().to_string();
    let before = fixture::files(&state.root);
    assert_eq!(
        legacy::capture_project(&state, &id)
            .await
            .err()
            .unwrap()
            .to_string(),
        "PROJECT_SNAPSHOT_MISSING"
    );
    assert_eq!(fixture::files(&state.root), before);
    assert_eq!(
        state.project_access.status(&state.root, &id).unwrap().mode,
        "closed"
    );
}

#[tokio::test]
async fn malformed_sql_only_inline_legacy_copy_rejects_before_any_new_project_or_artifact_write() {
    for wrong_size in [true, false] {
        let temp = tempfile::tempdir().unwrap();
        let state = fixture::state(temp.path()).await;
        let project = fixture::legacy_inline(&state, wrong_size).await;
        assert!(!state.root.join(&project.id).exists());
        assert!(project.files[0].data.is_some());
        // This is valid bounded legacy metadata/history; rejection belongs to decoded bytes.
        let captured = legacy::capture_with_history(&state, &project.id)
            .await
            .unwrap();
        assert!(captured.project.files[0].data.is_some());
        let before = fixture::snapshot(&state, &project.id).await;
        assert!(
            copy_for_editing(&state, &project.id).await.is_err(),
            "wrong_size={wrong_size}: inline bytes must match declared size and present SHA"
        );
        assert_eq!(fixture::snapshot(&state, &project.id).await, before);
        assert!(!state.root.join(&project.id).exists());
        assert_eq!(
            state
                .project_access
                .status(&state.root, &project.id)
                .unwrap()
                .mode,
            "closed"
        );
    }
}
