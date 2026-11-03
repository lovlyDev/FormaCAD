//! Public API lifecycle and exact-backup checks, using synthetic V2 metadata.
use super::*;
use crate::{models::Project, project_history::HistoryState, project_storage_v2 as storage};

fn fixture(root: &Path) -> FolderRepository {
    let project: Project = serde_json::from_value(serde_json::json!({
        "schemaVersion":1,"id":uuid::Uuid::new_v4().to_string(),"name":"Public migration fixture",
        "units":"mm","agent":"codex","pinned":false,"createdAt":"2026-11-03T00:00:00Z",
        "updatedAt":"2026-11-03T00:00:00Z","currentRevision":null,"messages":[],
        "exports":[],"files":[],"revisions":[]
    }))
    .unwrap();
    let mut generation =
        storage::stage_with(root, &project, |_| panic!("no attachments in fixture")).unwrap();
    let archive = serde_json::to_vec(&serde_json::json!({"schemaVersion":1,
        "state": HistoryState::from_project(&project).unwrap(), "events": []}))
    .unwrap();
    storage::stage_history(root, &mut generation, &archive).unwrap();
    storage::publish(root, generation).unwrap();
    FolderRepository::open(root).unwrap()
}

#[tokio::test]
async fn public_migration_retains_exact_backup_and_project_identity_without_app_sql() {
    let root = tempfile::tempdir().unwrap();
    let backup_root = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    let source = fixture(root.path());
    let original = source.capture().unwrap();
    let before = std::fs::read(root.path().join("manifest.json")).unwrap();
    let archive = source.read_history(&original).unwrap().unwrap();
    let backup = V2Backup::create(&source, &original, backup_root.path()).unwrap();
    assert_eq!(
        backup.record().manifest_sha256,
        crate::artifacts::digest(&before)
    );
    assert_eq!(
        std::fs::read(backup_root.path().join("manifest.json")).unwrap(),
        before
    );
    assert_eq!(std::fs::read_dir(target.path()).unwrap().count(), 0);
    let report = backup.migrate(&source, &original, target.path()).await;
    assert_eq!(report.backup.project_id, original.project().id);
    let saved = match report.target.unwrap() {
        Publication::Committed(saved) => saved,
        _ => panic!("expected confirmed conversion"),
    };
    assert_eq!(
        serde_json::to_value(saved.project()).unwrap(),
        serde_json::to_value(original.project()).unwrap()
    );
    let converted =
        crate::project_repository::sqlite_folder::api::FolderProject::open(target.path())
            .await
            .unwrap();
    let view = converted.capture().await.unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&converted.read_history(&view).await.unwrap())
            .unwrap(),
        serde_json::from_slice::<serde_json::Value>(&archive).unwrap()
    );
    source.recheck(&original).unwrap();
    assert_eq!(
        std::fs::read(root.path().join("manifest.json")).unwrap(),
        before
    );
}

#[tokio::test]
async fn same_path_reopen_cannot_reconstruct_the_backup_original_authority() {
    let root = tempfile::tempdir().unwrap();
    let backup_root = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    let source = fixture(root.path());
    let original = source.capture().unwrap();
    let backup = V2Backup::create(&source, &original, backup_root.path()).unwrap();
    let before = std::fs::read(root.path().join("manifest.json")).unwrap();
    source.close();
    let reopened = FolderRepository::open(root.path()).unwrap();
    let other = reopened.capture().unwrap();
    let report = backup.migrate(&reopened, &other, target.path()).await;
    assert!(report.target.is_err());
    assert_eq!(
        report.backup.manifest_sha256,
        backup.record().manifest_sha256
    );
    assert_eq!(std::fs::read_dir(target.path()).unwrap().count(), 0);
    assert_eq!(
        std::fs::read(root.path().join("manifest.json")).unwrap(),
        before
    );
    FolderRepository::open(backup_root.path())
        .unwrap()
        .capture()
        .unwrap();
}
