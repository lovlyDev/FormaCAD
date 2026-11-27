use super::*;
use crate::project_repository::legacy::read_test_fixture as fixture;

#[tokio::test]
async fn managed_refresh_preserves_original_epoch_and_archive_scope() {
    let root = tempfile::tempdir().unwrap();
    let state = fixture::state(root.path()).await;
    let mut project = fixture::project(&state, 16).await;
    let pending = crate::artifacts::normalize(&mut project).unwrap();
    let baseline = crate::projects::persist(&state, project, pending, None)
        .await
        .unwrap();
    let original = RepositoryRead::capture_with_history(
        ProjectRepository::managed(&state, &baseline.id).unwrap(),
    )
    .await
    .unwrap();
    assert!(original.read_history().await.unwrap().is_some());
    let mut metadata = baseline.clone();
    metadata.name = "Metadata refresh winner".into();
    crate::projects::persist(&state, metadata, vec![], None)
        .await
        .unwrap();
    let before = fixture::logical(&state, &baseline.id).await;
    let files = fixture::files(root.path());
    assert!(original.recheck().await.is_err());
    original.recheck_source().await.unwrap();
    let latest = original.recapture_metadata().await.unwrap();
    assert_eq!(latest.project().name, "Metadata refresh winner");
    assert_eq!(original.project().name, baseline.name);
    assert_eq!(latest.project().current_revision, baseline.current_revision);
    assert_eq!(latest.project().revisions, baseline.revisions);
    assert!(latest.read_history().await.is_err()); // Metadata never inherits old archive authority.
    assert!(original.read_history().await.is_err());
    assert_eq!(fixture::logical(&state, &baseline.id).await, before);
    assert_eq!(fixture::files(root.path()), files);
    state.project_access.release(&baseline.id).unwrap();
    state
        .project_access
        .acquire(&state.root, &baseline.id)
        .unwrap();
    assert!(original.recapture_metadata().await.is_err());
    assert!(latest.recapture_metadata().await.is_err());
    let current =
        RepositoryRead::capture(ProjectRepository::managed(&state, &baseline.id).unwrap())
            .await
            .unwrap();
    current.recheck().await.unwrap();
    state.project_access.release(&baseline.id).unwrap();
}

fn asset_project() -> Project {
    serde_json::from_value(serde_json::json!({
        "schemaVersion":1,"id":uuid::Uuid::new_v4().to_string(),"name":"Original source",
        "units":"mm","agent":"codex","pinned":false,"createdAt":"2026-11-04T00:00:00Z",
        "updatedAt":"2026-11-04T00:00:00Z","currentRevision":null,"messages":[],"exports":[],
        "revisions":[],"files":[{"name":"fixture.pdf","kind":"attachment","size":5,"sha256":crate::artifacts::digest(b"bytes")}]
    })).unwrap()
}

#[tokio::test]
async fn v2_refresh_retains_original_root_and_request_family() {
    let root = tempfile::tempdir().unwrap();
    let project = asset_project();
    let staged =
        crate::project_storage_v2::stage_with(root.path(), &project, |_| Ok(b"bytes".to_vec()))
            .unwrap();
    crate::project_storage_v2::publish(root.path(), staged).unwrap();
    let original = RepositoryRead::capture(ProjectRepository::folder(root.path()).unwrap())
        .await
        .unwrap();
    let mut metadata = project.clone();
    metadata.name = "New V2 metadata".into();
    let staged =
        crate::project_storage_v2::stage_with(root.path(), &metadata, |_| Ok(b"bytes".to_vec()))
            .unwrap();
    crate::project_storage_v2::publish(root.path(), staged).unwrap();
    let files = fixture::files(root.path());
    assert!(original.recheck().await.is_err());
    original.recheck_source().await.unwrap();
    let latest = original.recapture_metadata().await.unwrap();
    assert_eq!(latest.project().name, metadata.name);
    assert_eq!(original.project().name, project.name);
    assert_eq!(
        latest.read_asset(&latest.project().files[0]).await.unwrap(),
        b"bytes"
    );
    assert_eq!(fixture::files(root.path()), files);
    latest.close();
    assert!(original.recapture_metadata().await.is_err());
    let reopened = RepositoryRead::capture(ProjectRepository::folder(root.path()).unwrap())
        .await
        .unwrap();
    reopened.recheck().await.unwrap();
    assert!(original.recapture_metadata().await.is_err());
}

#[cfg(any(
    windows,
    all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )
))]
#[tokio::test]
async fn fcp1_refresh_preserves_opening_and_never_promotes_the_old_view() {
    use crate::project_repository::sqlite_folder::api::{FolderProject, Publication};
    let root = tempfile::tempdir().unwrap();
    assert!(matches!(
        FolderProject::create(
            root.path(),
            asset_project(),
            vec![(crate::artifacts::digest(b"bytes"), b"bytes".to_vec())],
            None
        )
        .await
        .unwrap(),
        Publication::Committed(_)
    ));
    let source = Arc::new(FolderProject::open(root.path()).await.unwrap());
    let original = RepositoryRead::from_folder_view(
        Arc::clone(&source),
        source.capture_metadata().await.unwrap(),
    )
    .await
    .unwrap();
    let view = source.capture_metadata().await.unwrap();
    let approval = source.approve_write(&view).await.unwrap();
    let mut metadata = view.project().clone();
    metadata.name = "New FCP1 metadata".into();
    assert!(matches!(
        source
            .commit(&view, approval, metadata.clone(), vec![])
            .await
            .unwrap(),
        Publication::Committed(_)
    ));
    let before = std::fs::read(root.path().join("project.sqlite")).unwrap();
    assert!(original.recheck().await.is_err());
    original.recheck_source().await.unwrap();
    let latest = original.recapture_metadata().await.unwrap();
    assert_eq!(latest.project().name, metadata.name);
    assert_ne!(original.project().name, latest.project().name);
    assert_eq!(latest.project().files, original.project().files);
    assert_eq!(latest.task_scope_id(), original.task_scope_id());
    assert!(original
        .read_asset(&original.project().files[0])
        .await
        .is_err());
    assert_eq!(
        latest.read_asset(&latest.project().files[0]).await.unwrap(),
        b"bytes"
    );
    assert_eq!(
        std::fs::read(root.path().join("project.sqlite")).unwrap(),
        before
    );
    latest.close();
    assert!(original.recheck_source().await.is_err());
    // A separate request over the still-live source remains valid.
    let another = RepositoryRead::from_folder_view(
        Arc::clone(&source),
        source.capture_metadata().await.unwrap(),
    )
    .await
    .unwrap();
    another.recheck().await.unwrap();
    source.close();
    assert!(another.recapture_metadata().await.is_err());
    let reopened = FolderProject::open(root.path()).await.unwrap();
    reopened.capture_metadata().await.unwrap();
    assert!(another.recapture_metadata().await.is_err());
    assert!(original.recapture_metadata().await.is_err());
}
