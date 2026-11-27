//! Actual folder openings and RAII reservations, without a synthetic authority setter.
use super::*;
use crate::{models::Project, project_repository::sqlite_folder::api::Publication};

#[test]
fn indexed_opening_and_creation_futures_can_run_in_tauri_commands() {
    fn require_send(_: impl Send) {}
    let root = Path::new("folder");
    let sources = FolderSources::with_index(FolderIndex::in_app_data(root));
    require_send(sources.open(root));
    require_send(sources.create_blank(root, "Indexed".into(), "mm".into(), "codex".into()));
}

#[tokio::test]
async fn successful_folder_creation_and_opening_update_recents() {
    let app = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let index = FolderIndex::in_app_data(app.path());
    let sources = FolderSources::with_index(FolderIndex::in_app_data(app.path()));
    let created = sources
        .create_blank(root.path(), "Indexed".into(), "mm".into(), "codex".into())
        .await
        .unwrap();
    let FolderCreation::Opened(created) = created else {
        panic!("expected live source");
    };
    let recent = index.recent().await.unwrap();
    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0].location, root.path());
    assert!(sources.close(&created.id.to_string()).unwrap());
    assert!(index.set_pinned(root.path(), true).await.unwrap());
    let reopened = sources.open(root.path()).await.unwrap();
    assert_eq!(reopened.snapshot.commit_id(), created.snapshot.commit_id());
    let recent = index.recent().await.unwrap();
    assert_eq!(recent.len(), 1);
    assert!(recent[0].pinned);
}

#[tokio::test]
async fn failing_index_does_not_prevent_folder_opening() {
    let app = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let project_id = uuid::Uuid::new_v4().to_string();
    fixture(root.path(), &project_id).await;
    let missing = app.path().join("missing-app-index-directory");
    let sources = FolderSources::with_index(FolderIndex::in_app_data(&missing));
    let opened = sources.open(root.path()).await.unwrap();
    assert_eq!(opened.snapshot.project().id, project_id);
    sources
        .lookup(&opened.id.to_string())
        .unwrap()
        .recheck(&opened.snapshot)
        .await
        .unwrap();
    assert!(!missing.exists());
}

#[tokio::test]
async fn blank_creation_opens_exact_saved_commit_and_reopens_without_window_registry() {
    let root = tempfile::tempdir().unwrap();
    let sources = FolderSources::default();
    let created = sources
        .create_blank(
            root.path(),
            "  New box  ".into(),
            "cm".into(),
            "codex".into(),
        )
        .await
        .unwrap();
    let FolderCreation::Opened(opened) = created else {
        panic!("expected a live opening for the saved project")
    };
    let project = opened.snapshot.project();
    assert_eq!(project.name, "New box");
    assert_eq!(project.units, "cm");
    assert_eq!(project.current_revision, None);
    assert!(project.revisions.is_empty());
    assert!(project.files.is_empty());
    let key = opened.id.to_string();
    let source = sources.lookup(&key).unwrap();
    source.recheck(&opened.snapshot).await.unwrap();
    assert!(sources.close(&key).unwrap());
    assert!(source.recheck(&opened.snapshot).await.is_err());
    let independent = FolderSources::default();
    let reopened = independent.open(root.path()).await.unwrap();
    assert_ne!(reopened.id, opened.id);
    assert_eq!(reopened.snapshot.commit_id(), opened.snapshot.commit_id());
    assert_eq!(reopened.snapshot.project().id, project.id);
}

#[tokio::test]
async fn blank_creation_refuses_invalid_inputs_nonempty_folders_and_closed_windows() {
    let root = tempfile::tempdir().unwrap();
    let sources = FolderSources::default();
    let create = |name: &str| {
        sources.create_blank(root.path(), name.to_owned(), "mm".into(), "codex".into())
    };
    assert!(create("   ").await.is_err());
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);

    let existing = root.path().join("existing.step");
    std::fs::write(&existing, b"User data").unwrap();
    assert!(create("Existing").await.is_err());
    assert_eq!(std::fs::read(&existing).unwrap(), b"User data");
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
    std::fs::remove_file(existing).unwrap();

    sources.close_all().unwrap();
    assert!(create("Closed").await.is_err());
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn common_readers_keep_original_window_source_and_close_only_their_own_handle() {
    use crate::project_repository::backend::ProjectRepository;
    let root = tempfile::tempdir().unwrap();
    fixture(root.path(), &uuid::Uuid::new_v4().to_string()).await;
    let sources = FolderSources::default();
    let opened = sources.open(root.path()).await.unwrap();
    let key = opened.id.to_string();
    let first = ProjectRepository::from_folder_source(sources.lookup(&key).unwrap());
    let second = ProjectRepository::from_folder_source(sources.lookup(&key).unwrap());
    let first_view = first.capture().await.unwrap();
    let second_view = second.capture_with_history().await.unwrap();
    assert!(second.recheck(&first_view).await.is_err());
    assert!(first.recheck(&second_view).await.is_err());
    assert!(second.read_history(&second_view).await.unwrap().is_some());
    first.close();
    assert!(first.recheck(&first_view).await.is_err());
    second.recheck(&second_view).await.unwrap();
    sources
        .lookup(&key)
        .unwrap()
        .recheck(&opened.snapshot)
        .await
        .unwrap();
    assert!(sources.close(&key).unwrap());
    assert!(second.recheck(&second_view).await.is_err());
    assert!(second.capture().await.is_err());
    assert!(second.read_history(&second_view).await.is_err());
    let reopened = sources.open(root.path()).await.unwrap();
    let third =
        ProjectRepository::from_folder_source(sources.lookup(&reopened.id.to_string()).unwrap());
    assert!(third.recheck(&second_view).await.is_err());
    third.capture().await.unwrap();
    assert!(second.capture().await.is_err());
}

async fn fixture(root: &Path, id: &str) {
    let project: Project = serde_json::from_value(serde_json::json!({
        "schemaVersion":1,"id":id,"name":"Source registry fixture",
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
async fn same_project_ids_and_reopen_never_substitute_original_source() {
    let first_root = tempfile::tempdir().unwrap();
    let second_root = tempfile::tempdir().unwrap();
    let project_id = uuid::Uuid::new_v4().to_string();
    fixture(first_root.path(), &project_id).await;
    fixture(second_root.path(), &project_id).await;
    let sources = FolderSources::default();
    let first = sources.open(first_root.path()).await.unwrap();
    let second = sources.open(second_root.path()).await.unwrap();
    assert_eq!(first.snapshot.project().id, second.snapshot.project().id);
    assert_ne!(first.id, second.id);
    let a = sources.lookup(&first.id.to_string()).unwrap();
    let b = sources.lookup(&second.id.to_string()).unwrap();
    assert!(b.recheck(&first.snapshot).await.is_err());
    assert!(sources.close(&first.id.to_string()).unwrap());
    assert!(sources.lookup(&first.id.to_string()).is_err());
    assert!(a.recheck(&first.snapshot).await.is_err());
    b.recheck(&second.snapshot).await.unwrap();
    assert!(!sources.close(&first.id.to_string()).unwrap());
    let reopened = sources.open(first_root.path()).await.unwrap();
    assert_ne!(first.id, reopened.id);
    assert!(reopened.id.sequence > first.id.sequence);
    assert!(sources
        .lookup(&reopened.id.to_string())
        .unwrap()
        .recheck(&first.snapshot)
        .await
        .is_err());
}

#[tokio::test]
async fn forged_nonce_foreign_scope_and_paths_cannot_lookup_or_close_a_source() {
    let root = tempfile::tempdir().unwrap();
    fixture(root.path(), &uuid::Uuid::new_v4().to_string()).await;
    let sources = FolderSources::default();
    let other_window = FolderSources::default();
    let opened = sources.open(root.path()).await.unwrap();
    let forged = format!(
        "{}:{}:{}",
        opened.id.scope,
        opened.id.sequence,
        uuid::Uuid::new_v4()
    );
    assert!(sources.lookup(&forged).is_err());
    assert!(!sources.close(&forged).unwrap());
    assert!(other_window.lookup(&opened.id.to_string()).is_err());
    assert!(other_window.close(&opened.id.to_string()).is_err());
    assert!(sources.lookup("../project.sqlite").is_err());
    assert!(sources.lookup(&"x".repeat(129)).is_err());
    sources
        .lookup(&opened.id.to_string())
        .unwrap()
        .recheck(&opened.snapshot)
        .await
        .unwrap();
}

#[tokio::test]
async fn window_close_and_drop_revoke_inflight_arcs_and_pending_publication() {
    let root = tempfile::tempdir().unwrap();
    fixture(root.path(), &uuid::Uuid::new_v4().to_string()).await;
    let sources = FolderSources::default();
    let opened = sources.open(root.path()).await.unwrap();
    let retained = sources.lookup(&opened.id.to_string()).unwrap();
    let mut pending = sources.reserve().unwrap();
    sources.close_all().unwrap();
    assert!(retained.recheck(&opened.snapshot).await.is_err());
    assert!(sources.lookup(&opened.id.to_string()).is_err());
    assert!(pending
        .publish(Arc::new(FolderProject::open(root.path()).await.unwrap()))
        .is_err());
    drop(pending);
    assert!(sources.open(root.path()).await.is_err());
    let next_window = FolderSources::default();
    let next = next_window.open(root.path()).await.unwrap();
    let retained = next_window.lookup(&next.id.to_string()).unwrap();
    drop(next_window);
    assert!(retained.recheck(&next.snapshot).await.is_err());
}

#[tokio::test]
async fn pending_reservations_limit_before_io_and_drop_returns_capacity() {
    let root = tempfile::tempdir().unwrap();
    fixture(root.path(), &uuid::Uuid::new_v4().to_string()).await;
    let sources = FolderSources::default();
    let mut pending: Vec<_> = (0..MAX_FOLDER_SOURCES)
        .map(|_| sources.reserve().unwrap())
        .collect();
    assert!(sources.state.try_lock().is_ok());
    let missing = root.path().join("not-a-source");
    assert!(
        matches!(sources.open(&missing).await, Err(AppError::Invalid(code)) if code == "PROJECT_SOURCE_LIMIT")
    );
    drop(pending.pop().unwrap());
    let opened = sources.open(root.path()).await.unwrap();
    drop(pending);
    assert!(sources.state.lock().unwrap().pending.is_empty());
    assert!(sources.open(&missing).await.is_err());
    assert!(sources.state.lock().unwrap().pending.is_empty());
    sources
        .lookup(&opened.id.to_string())
        .unwrap()
        .recheck(&opened.snapshot)
        .await
        .unwrap();
}

#[tokio::test]
async fn cancellation_after_publication_revokes_only_unacknowledged_source() {
    let root = tempfile::tempdir().unwrap();
    fixture(root.path(), &uuid::Uuid::new_v4().to_string()).await;
    let sources = FolderSources::default();
    let existing = sources.open(root.path()).await.unwrap();
    let mut opening = sources.reserve().unwrap();
    let source = Arc::new(FolderProject::open(root.path()).await.unwrap());
    let snapshot = source.capture_metadata().await.unwrap();
    opening.publish(Arc::clone(&source)).unwrap();
    let unacknowledged = opening.id.to_string();
    sources
        .lookup(&unacknowledged)
        .unwrap()
        .recheck(&snapshot)
        .await
        .unwrap();
    assert!(sources.state.try_lock().is_ok());
    // The real guard's Drop is also what runs if the open future is cancelled.
    drop(opening);
    assert!(sources.lookup(&unacknowledged).is_err());
    assert!(source.recheck(&snapshot).await.is_err());
    sources
        .lookup(&existing.id.to_string())
        .unwrap()
        .recheck(&existing.snapshot)
        .await
        .unwrap();
}
