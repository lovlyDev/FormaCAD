use super::*;

#[tokio::test]
async fn records_locations_and_pins_without_requiring_access_to_the_folder() {
    let app_data = tempfile::tempdir().unwrap();
    let index = FolderIndex::in_app_data(app_data.path());
    let missing = app_data.path().join("folder-disconnected");
    let other = app_data.path().join("other-disconnected-folder");
    assert!(index.recent().await.unwrap().is_empty());
    assert!(!index.path.exists());
    let first = index.record_open(&missing);
    let second = index.record_open(&other);
    let (first, second) = tokio::join!(first, second);
    first.unwrap();
    second.unwrap();
    assert!(index.set_pinned(&missing, true).await.unwrap());
    index.record_open(&missing).await.unwrap();
    assert!(!index
        .set_pinned(&app_data.path().join("unknown"), true)
        .await
        .unwrap());
    let reopened = FolderIndex::in_app_data(app_data.path());
    let recent = reopened.recent().await.unwrap();
    assert_eq!(recent.len(), 2);
    assert_eq!(recent[0].location, missing);
    assert!(recent[0].pinned);
    assert_eq!(recent[1].location, other);
}

#[tokio::test]
async fn disabling_recents_is_persisted_without_adding_locations() {
    let app_data = tempfile::tempdir().unwrap();
    let index = FolderIndex::in_app_data(app_data.path());
    assert!(index.remember_recent().await.unwrap());
    index.set_remember_recent(false).await.unwrap();
    let reopened = FolderIndex::in_app_data(app_data.path());
    assert!(!reopened.remember_recent().await.unwrap());
    reopened
        .record_open(&app_data.path().join("not-remembered"))
        .await
        .unwrap();
    assert!(reopened.recent().await.unwrap().is_empty());
}

#[tokio::test]
async fn refuses_an_unrelated_sqlite_file_without_changing_it() {
    let app_data = tempfile::tempdir().unwrap();
    let index = FolderIndex::in_app_data(app_data.path());
    let mut db = index.connect(false).await.unwrap();
    sqlx::query("CREATE TABLE private_data (data TEXT)")
        .execute(&mut db)
        .await
        .unwrap();
    db.close().await.unwrap();
    let before = std::fs::read(&index.path).unwrap();
    assert!(index
        .record_open(&app_data.path().join("folder"))
        .await
        .is_err());
    assert!(index.recent().await.is_err());
    assert_eq!(before, std::fs::read(&index.path).unwrap());
}

#[cfg(any(
    windows,
    all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )
))]
#[tokio::test]
async fn deleting_the_app_index_leaves_the_folder_commit_reopenable() {
    use crate::project_repository::sqlite_folder::api::{FolderCreation, FolderSources};

    let app_data = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let sources = FolderSources::default();
    let created = sources
        .create_blank(
            root.path(),
            "Independent".into(),
            "mm".into(),
            "codex".into(),
        )
        .await
        .unwrap();
    let FolderCreation::Opened(created) = created else {
        panic!("expected creation and opening");
    };
    let project_id = &created.snapshot.project().id;
    let commit_id = created.snapshot.commit_id();
    let db_path = root.path().join("project.sqlite");
    let initial = std::fs::read(&db_path).unwrap();
    let index = FolderIndex::in_app_data(app_data.path());
    index.record_open(root.path()).await.unwrap();
    assert!(index.set_pinned(root.path(), true).await.unwrap());
    assert_eq!(index.recent().await.unwrap().len(), 1);
    assert!(sources.close(&created.id.to_string()).unwrap());
    assert_eq!(initial, std::fs::read(&db_path).unwrap());

    std::fs::remove_file(&index.path).unwrap();
    let reopened_index = FolderIndex::in_app_data(app_data.path());
    assert!(reopened_index.recent().await.unwrap().is_empty());
    assert!(reopened_index.remember_recent().await.unwrap());
    let fresh_sources = FolderSources::default();
    let reopened = fresh_sources.open(root.path()).await.unwrap();
    assert_eq!(reopened.snapshot.project().id, *project_id);
    assert_eq!(reopened.snapshot.commit_id(), commit_id);
    assert_eq!(initial, std::fs::read(&db_path).unwrap());
    assert!(!reopened_index.path.exists());
}
