//! Lazy capture stays pure; history/copy/write must validate the original archive.
use super::*;
use crate::project_repository::backend::ProjectRepository;
use sqlx::{sqlite::SqliteConnectOptions, Connection};

async fn fixture(root: &Path) {
    let project: Project = serde_json::from_value(serde_json::json!({
        "schemaVersion":1,"id":uuid::Uuid::new_v4().to_string(),"name":"Lazy API fixture",
        "units":"mm","agent":"codex","pinned":false,"createdAt":"2026-11-04T00:00:00Z",
        "updatedAt":"2026-11-04T00:00:00Z","currentRevision":null,"messages":[],
        "exports":[],"files":[],"revisions":[]
    }))
    .unwrap();
    assert!(matches!(
        FolderProject::create(root, project, vec![], None)
            .await
            .unwrap(),
        Publication::Committed(_)
    ));
}

#[tokio::test]
async fn metadata_does_not_adopt_unverified_archive_as_write_copy_or_history_authority() {
    let root = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    fixture(root.path()).await;
    // Simulate external damage to events without changing the declared commit seal.
    let mut external = sqlx::SqliteConnection::connect_with(
        &SqliteConnectOptions::new().filename(root.path().join("project.sqlite")),
    )
    .await
    .unwrap();
    let project_id: String = sqlx::query_scalar("SELECT project_id FROM repository_state")
        .fetch_one(&mut external)
        .await
        .unwrap();
    sqlx::query("INSERT INTO project_history_events(project_id,kind,from_revision,to_revision,target_revision,created_at) VALUES(?,'edit',NULL,?,NULL,'2026-11-04T00:00:00Z')")
        .bind(&project_id).bind(uuid::Uuid::new_v4().to_string()).execute(&mut external).await.unwrap();
    external.close().await.unwrap();
    let before = std::fs::read(root.path().join("project.sqlite")).unwrap();
    let source = FolderProject::open(root.path()).await.unwrap();
    let metadata = source.capture_metadata().await.unwrap();
    assert_eq!(metadata.project().id, project_id);
    source.recheck(&metadata).await.unwrap();
    assert!(source.capture().await.is_err());
    assert!(source.read_history(&metadata).await.is_err());
    assert!(source.approve_write(&metadata).await.is_err());
    assert!(source.save_copy(&metadata, target.path()).await.is_err());
    assert_eq!(std::fs::read_dir(target.path()).unwrap().count(), 0);
    let common = ProjectRepository::independent_folder(root.path())
        .await
        .unwrap();
    let snapshot = common.capture().await.unwrap();
    assert_eq!(snapshot.project().id, project_id);
    assert!(common.capture_with_history().await.is_err());
    assert!(common.read_history(&snapshot).await.is_err());
    assert_eq!(
        before,
        std::fs::read(root.path().join("project.sqlite")).unwrap()
    );
    source.close();
    assert!(source.read_history(&metadata).await.is_err());
}

#[tokio::test]
async fn deferred_archive_read_compares_original_saved_state_instead_of_capturing_a_new_one() {
    let root = tempfile::tempdir().unwrap();
    fixture(root.path()).await;
    let source = FolderProject::open(root.path()).await.unwrap();
    let original = source.capture_metadata().await.unwrap();
    let writer = FolderProject::open(root.path()).await.unwrap();
    let current = writer.capture().await.unwrap();
    let approval = writer.approve_write(&current).await.unwrap();
    let mut next = current.project().clone();
    next.name = "New external saved state".into();
    assert!(matches!(
        writer
            .commit(&current, approval, next, vec![])
            .await
            .unwrap(),
        Publication::Committed(_)
    ));
    assert!(source.recheck(&original).await.is_err());
    assert!(source.read_history(&original).await.is_err());
    assert!(source.approve_write(&original).await.is_err());
    let fresh = source.capture_metadata().await.unwrap();
    assert_ne!(fresh.commit_id(), original.commit_id());
    let history = source.read_history(&fresh).await.unwrap();
    crate::project_history::validate_archive(fresh.project(), &history).unwrap();
    source.recheck(&fresh).await.unwrap();
}
