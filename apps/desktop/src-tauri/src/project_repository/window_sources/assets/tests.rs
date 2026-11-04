use super::*;
use crate::{
    models::Project,
    project_repository::sqlite_folder::api::{FolderProject, Publication},
};

async fn fixture(root: &std::path::Path, id: &str, bytes: &[u8]) {
    let hash = crate::artifacts::digest(bytes);
    let project: Project = serde_json::from_value(serde_json::json!({
        "schemaVersion":1,"id":id,"name":"Bound asset fixture","units":"mm","agent":"codex","pinned":false,
        "createdAt":"2026-11-04T00:00:00Z","updatedAt":"2026-11-04T00:00:00Z","currentRevision":null,
        "messages":[],"exports":[],"files":[{"name":"fixture.pdf","kind":"attachment","size":bytes.len(),"sha256":hash}],"revisions":[]
    })).unwrap();
    assert!(matches!(
        FolderProject::create(root, project, vec![(hash, bytes.to_vec())], None)
            .await
            .unwrap(),
        Publication::Committed(_)
    ));
}

#[tokio::test]
async fn assets_bind_original_source_commit_and_descriptor_without_app_index() {
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    fixture(a.path(), &id, b"source-A bytes").await;
    fixture(b.path(), &id, b"source-B bytes").await;
    let windows = WindowSources::default();
    let scope = windows.for_window("assets").unwrap();
    let first = scope.open(a.path()).await.unwrap();
    let second = scope.open(b.path()).await.unwrap();
    let key_a = first.id.to_string();
    let key_b = second.id.to_string();
    let commit_a = first.snapshot.commit_id();
    let commit_b = second.snapshot.commit_id();
    let before = std::fs::read(a.path().join("project.sqlite")).unwrap();
    assert_eq!(
        windows
            .read_asset("assets", &key_a, &id, commit_a, "fixture.pdf")
            .await
            .unwrap(),
        crate::artifacts::data_url("fixture.pdf", b"source-A bytes")
    );
    assert_eq!(
        windows
            .read_asset("assets", &key_b, &id, commit_b, "fixture.pdf")
            .await
            .unwrap(),
        crate::artifacts::data_url("fixture.pdf", b"source-B bytes")
    );
    assert!(windows
        .read_asset("assets", &key_a, &id, commit_b, "fixture.pdf")
        .await
        .is_err());
    assert!(windows
        .read_asset("assets", &key_a, &id, commit_a, "missing.pdf")
        .await
        .is_err());
    assert!(windows
        .read_asset("assets", &key_a, &id, commit_a, "../fixture.pdf")
        .await
        .is_err());
    assert_eq!(
        before,
        std::fs::read(a.path().join("project.sqlite")).unwrap()
    );
    let source = scope.lookup(&key_a).unwrap();
    let foreign_view = source.capture_metadata().await.unwrap();
    assert!(
        crate::project_repository::backend::RepositoryRead::from_folder_view(
            scope.lookup(&key_b).unwrap(),
            foreign_view
        )
        .await
        .is_err()
    );
    let closing_read = windows
        .capture_read("assets", &key_a, &id, commit_a)
        .await
        .unwrap();
    let stale_read = windows
        .capture_read("assets", &key_a, &id, commit_a)
        .await
        .unwrap();
    assert!(stale_read.read_history().await.unwrap().is_some());
    closing_read.close();
    assert!(closing_read.recheck().await.is_err());
    stale_read.recheck().await.unwrap();
    assert_eq!(
        windows
            .read_asset("assets", &key_a, &id, commit_a, "fixture.pdf")
            .await
            .unwrap(),
        crate::artifacts::data_url("fixture.pdf", b"source-A bytes")
    );
    let approval = source.approve_write(&first.snapshot).await.unwrap();
    let mut changed = first.snapshot.project().clone();
    changed.name = "Externally changed metadata".into();
    assert!(matches!(
        source
            .commit(&first.snapshot, approval, changed, vec![])
            .await
            .unwrap(),
        Publication::Committed(_)
    ));
    assert!(stale_read.recheck().await.is_err());
    assert!(stale_read.read_history().await.is_err());
    assert!(windows
        .read_asset("assets", &key_a, &id, commit_a, "fixture.pdf")
        .await
        .is_err());
    let closing_source = windows
        .capture_read("assets", &key_b, &id, commit_b)
        .await
        .unwrap();
    assert!(windows.close_source("assets", &key_b).unwrap());
    assert!(closing_source.recheck().await.is_err());
    assert!(windows
        .read_asset("assets", &key_b, &id, commit_b, "fixture.pdf")
        .await
        .is_err());
}
