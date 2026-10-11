//! Real SQL and filesystem fences for raw saved-STEP manual sections.
use super::{fixture, plane, store};
use forma_core::core::AppState;
use std::{
    path::Path,
    sync::{atomic::AtomicBool, Arc},
};

async fn payload(state: &AppState, id: &str) -> String {
    sqlx::query_scalar("SELECT payload FROM projects WHERE id=?")
        .bind(id)
        .fetch_one(&state.pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn recovery_journal_is_rejected_without_publication_or_cleanup() {
    let root = tempfile::tempdir().unwrap();
    let (state, project) = fixture(root.path()).await;
    let before = payload(&state, &project.id).await;
    let journal = "private pending generation must remain unparsed and unchanged";
    sqlx::query("INSERT INTO project_storage_commits(project_id,base_manifest_sha256,target_manifest,project_sha256) VALUES(?,NULL,?,?)")
        .bind(&project.id).bind(journal).bind("pending-seal").execute(&state.pool).await.unwrap();
    let error = forma_core::model_section::build::section_with_executable(
        &state,
        &project.id,
        project.current_revision.as_deref().unwrap(),
        &plane(),
        Arc::new(AtomicBool::new(false)),
        Path::new("worker-must-not-run.exe"),
    )
    .await
    .unwrap_err();
    assert!(error
        .to_string()
        .contains("PROJECT_SNAPSHOT_RECOVERY_REQUIRED"));
    assert_eq!(payload(&state, &project.id).await, before);
    let after: String = sqlx::query_scalar(
        "SELECT target_manifest FROM project_storage_commits WHERE project_id=?",
    )
    .bind(&project.id)
    .fetch_one(&state.pool)
    .await
    .unwrap();
    assert_eq!(after, journal);
    assert_eq!(
        std::fs::read(root.path().join(&project.id).join("sentinel.txt")).unwrap(),
        b"read-only project"
    );
    assert_eq!(
        std::fs::read_dir(root.path().join(&project.id))
            .unwrap()
            .count(),
        1
    );
    assert!(!root.path().join(".transient/sections").exists());
    assert!(state.cad_tasks.status().unwrap().is_empty());
}

#[tokio::test]
async fn unchanged_head_with_mutated_original_step_during_queue_is_rejected() {
    let root = tempfile::tempdir().unwrap();
    let (state, mut project) = fixture(root.path()).await;
    let bytes = std::fs::read(root.path().join("fixture.step")).unwrap();
    let hash = project.files[0].sha256.clone().unwrap();
    let attachments = root.path().join(&project.id).join("attachments");
    std::fs::create_dir(&attachments).unwrap();
    let original = attachments.join(format!("{hash}.step"));
    std::fs::write(&original, &bytes).unwrap();
    project.files[0].data = None;
    store(&state, &project).await;
    let before = payload(&state, &project.id).await;
    let blocker = state
        .cad_tasks
        .acquire(
            &project.id,
            "blocking-test",
            Arc::new(AtomicBool::new(false)),
        )
        .await
        .unwrap();
    let task_state = state.clone();
    let id = project.id.clone();
    let revision = project.current_revision.clone().unwrap();
    let task = tokio::spawn(async move {
        forma_core::model_section::build::section_with_executable(
            &task_state,
            &id,
            &revision,
            &plane(),
            Arc::new(AtomicBool::new(false)),
            Path::new("worker-must-not-run.exe"),
        )
        .await
    });
    for _ in 0..100 {
        if state
            .cad_tasks
            .status()
            .unwrap()
            .iter()
            .any(|task| task.kind == "section")
        {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    assert!(state
        .cad_tasks
        .status()
        .unwrap()
        .iter()
        .any(|task| task.kind == "section"));
    let mut changed = bytes.clone();
    changed[0] ^= 1;
    std::fs::write(&original, &changed).unwrap();
    drop(blocker);
    let error = task.await.unwrap().unwrap_err();
    assert!(error.to_string().contains("ASSET_CHECKSUM_MISMATCH"));
    assert_eq!(payload(&state, &project.id).await, before);
    assert_eq!(std::fs::read(&original).unwrap(), changed);
    assert!(!root.path().join(".transient/sections").exists());
    assert!(state.cad_tasks.status().unwrap().is_empty());
}

#[tokio::test]
async fn unchanged_head_with_changed_sql_attachment_seal_during_queue_is_stale() {
    let root = tempfile::tempdir().unwrap();
    let (state, project) = fixture(root.path()).await;
    let blocker = state
        .cad_tasks
        .acquire(
            &project.id,
            "blocking-test",
            Arc::new(AtomicBool::new(false)),
        )
        .await
        .unwrap();
    let task_state = state.clone();
    let id = project.id.clone();
    let revision = project.current_revision.clone().unwrap();
    let task = tokio::spawn(async move {
        forma_core::model_section::build::section_with_executable(
            &task_state,
            &id,
            &revision,
            &plane(),
            Arc::new(AtomicBool::new(false)),
            Path::new("worker-must-not-run.exe"),
        )
        .await
    });
    for _ in 0..100 {
        if state
            .cad_tasks
            .status()
            .unwrap()
            .iter()
            .any(|task| task.kind == "section")
        {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    assert!(state
        .cad_tasks
        .status()
        .unwrap()
        .iter()
        .any(|task| task.kind == "section"));
    let mut changed = project.clone();
    changed.files[0].sha256 = Some("0".repeat(64));
    assert_eq!(changed.current_revision, project.current_revision);
    store(&state, &changed).await;
    let after_writer = payload(&state, &project.id).await;
    drop(blocker);
    let error = task.await.unwrap().unwrap_err();
    assert!(error.to_string().contains("SECTION_STALE"));
    assert_eq!(payload(&state, &project.id).await, after_writer);
    assert_eq!(
        std::fs::read_dir(root.path().join(&project.id))
            .unwrap()
            .count(),
        1
    );
    assert!(!root.path().join(".transient/sections").exists());
    assert!(state.cad_tasks.status().unwrap().is_empty());
}
