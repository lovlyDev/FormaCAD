use super::*;
use crate::{
    models::Project,
    project_repository::sqlite_folder::api::{FolderProject, Publication},
};

#[tokio::test]
async fn windows_and_reused_labels_cannot_adopt_each_others_openings() {
    let root = tempfile::tempdir().unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    let project: Project = serde_json::from_value(serde_json::json!({
        "schemaVersion":1,"id":id,"name":"Window source fixture","units":"mm","agent":"codex","pinned":false,
        "createdAt":"2026-11-04T00:00:00Z","updatedAt":"2026-11-04T00:00:00Z","currentRevision":null,
        "messages":[],"exports":[],"files":[],"revisions":[]
    })).unwrap();
    assert!(matches!(
        FolderProject::create(root.path(), project, vec![], None)
            .await
            .unwrap(),
        Publication::Committed(_)
    ));
    let windows = WindowSources::default();
    let first = windows.for_window("first").unwrap();
    let opened = first.open(root.path()).await.unwrap();
    let key = opened.id.to_string();
    let commit = opened.snapshot.commit_id();
    windows.read("first", &key, &id, commit).await.unwrap();
    windows.for_window("second").unwrap();
    assert!(windows.read("second", &key, &id, commit).await.is_err());
    assert!(windows.close_source("second", &key).is_err());
    assert!(windows
        .read("first", &key, &id, "wrong-commit")
        .await
        .is_err());
    assert!(windows
        .read("first", &key, "wrong-project", commit)
        .await
        .is_err());
    windows.close_window("first");
    assert!(first.open(root.path()).await.is_err());
    assert!(windows.read("first", &key, &id, commit).await.is_err());
    let replacement = windows.for_window("first").unwrap();
    assert!(!Arc::ptr_eq(&first, &replacement));
    assert!(windows.read("first", &key, &id, commit).await.is_err());
    let current = replacement.open(root.path()).await.unwrap();
    let current_key = current.id.to_string();
    windows
        .read("first", &current_key, &id, current.snapshot.commit_id())
        .await
        .unwrap();
    drop(windows);
    assert!(replacement.lookup(&current_key).is_err());
}
