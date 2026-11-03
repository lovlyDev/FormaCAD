//! Real isolated SQL/CAS and opaque host authority checks, not CAD geometry proof.
use super::*;

fn project() -> Project {
    serde_json::from_value(serde_json::json!({
        "schemaVersion":1,"id":uuid::Uuid::new_v4().to_string(),"name":"Host API fixture",
        "units":"mm","agent":"codex","pinned":false,"createdAt":"2026-11-03T00:00:00Z",
        "updatedAt":"2026-11-03T00:00:00Z","currentRevision":null,"messages":[],
        "exports":[],"files":[],"revisions":[]
    }))
    .unwrap()
}

#[tokio::test]
async fn common_read_contract_uses_the_selected_folder_and_revokes_only_its_handle() {
    use crate::project_repository::backend::ProjectRepository;
    let root = tempfile::tempdir().unwrap();
    let mut initial = project();
    let bytes = b"synthetic bounded attachment, not CAD geometry";
    let hash = crate::artifacts::digest(bytes);
    initial.files.push(ProjectFile {
        name: "fixture.step".into(),
        size: bytes.len() as u64,
        kind: "model".into(),
        data: None,
        sha256: Some(hash.clone()),
    });
    committed(
        FolderProject::create(root.path(), initial, vec![(hash, bytes.to_vec())], None)
            .await
            .unwrap(),
    );
    let before = std::fs::read(root.path().join("project.sqlite")).unwrap();
    let source = ProjectRepository::independent_folder(root.path())
        .await
        .unwrap();
    let original = source.capture_with_history().await.unwrap();
    assert_eq!(
        source
            .read_asset(&original, &original.project().files[0])
            .await
            .unwrap(),
        bytes
    );
    let archive = source.read_history(&original).await.unwrap().unwrap();
    crate::project_history::validate_archive(original.project(), archive.as_ref()).unwrap();
    let second = ProjectRepository::independent_folder(root.path())
        .await
        .unwrap();
    let other = second.capture().await.unwrap();
    assert_eq!(original.project().id, other.project().id);
    assert!(second.recheck(&original).await.is_err());
    source.close();
    assert!(source.capture().await.is_err());
    assert!(source
        .read_asset(&original, &original.project().files[0])
        .await
        .is_err());
    assert!(source.read_history(&original).await.is_err());
    second.recheck(&other).await.unwrap();
    assert_eq!(
        before,
        std::fs::read(root.path().join("project.sqlite")).unwrap()
    );
}
fn committed(outcome: Publication) -> SavedCommit {
    match outcome {
        Publication::Committed(saved) => saved,
        _ => panic!("expected confirmed publication"),
    }
}

#[tokio::test]
async fn actual_host_api_opens_commits_closes_and_reopens_without_app_index() {
    let root = tempfile::tempdir().unwrap();
    let initial = project();
    let saved = committed(
        FolderProject::create(root.path(), initial.clone(), vec![], None)
            .await
            .unwrap(),
    );
    assert_eq!(saved.project().id, initial.id);
    let source = FolderProject::open(root.path()).await.unwrap();
    let original = source.capture().await.unwrap();
    assert_eq!(original.commit_id(), saved.commit_id());
    let history = source.read_history(&original).await.unwrap();
    crate::project_history::validate_archive(original.project(), &history).unwrap();
    assert_eq!(
        source
            .resolve_commit(&original, saved.commit_id())
            .await
            .unwrap(),
        CommitPresence::Present { is_head: true }
    );
    let approval = source.approve_write(&original).await.unwrap();
    let mut next = initial;
    next.name = "Explicit saved metadata".into();
    let receipt = committed(
        source
            .commit(&original, approval, next, vec![])
            .await
            .unwrap(),
    );
    assert!(source.recheck(&original).await.is_err());
    source.close();
    assert!(source.capture().await.is_err());
    drop(source);
    let reopened = FolderProject::open(root.path()).await.unwrap();
    let view = reopened.capture().await.unwrap();
    assert_eq!(view.commit_id(), receipt.commit_id());
    assert_eq!(view.project().name, "Explicit saved metadata");
    assert!(reopened.recheck(&original).await.is_err());
}

#[tokio::test]
async fn approval_and_read_view_cannot_be_reconstructed_from_another_opening() {
    let root = tempfile::tempdir().unwrap();
    committed(
        FolderProject::create(root.path(), project(), vec![], None)
            .await
            .unwrap(),
    );
    let first = FolderProject::open(root.path()).await.unwrap();
    let original = first.capture().await.unwrap();
    let second = FolderProject::open(root.path()).await.unwrap();
    let foreign = second.capture().await.unwrap();
    let approval = first.approve_write(&original).await.unwrap();
    let before = std::fs::read(root.path().join("project.sqlite")).unwrap();
    assert!(second
        .commit(&foreign, approval, foreign.project().clone(), vec![])
        .await
        .is_err());
    assert!(second.read_history(&original).await.is_err());
    assert!(second.approve_write(&original).await.is_err());
    assert_eq!(
        before,
        std::fs::read(root.path().join("project.sqlite")).unwrap()
    );
    first.recheck(&original).await.unwrap();
    second.recheck(&foreign).await.unwrap();
}

#[tokio::test]
async fn read_only_copy_succeeds_while_another_cooperative_writer_owns_source() {
    let root = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    committed(
        FolderProject::create(root.path(), project(), vec![], None)
            .await
            .unwrap(),
    );
    let source = FolderProject::open(root.path()).await.unwrap();
    let original = source.capture().await.unwrap();
    let before = std::fs::read(root.path().join("project.sqlite")).unwrap();
    let lease = crate::project_access::folder_lease::FolderLease::acquire(root.path()).unwrap();
    let saved = committed(source.save_copy(&original, target.path()).await.unwrap());
    assert_ne!(saved.project().id, original.project().id);
    assert_eq!(
        before,
        std::fs::read(root.path().join("project.sqlite")).unwrap()
    );
    source.recheck(&original).await.unwrap();
    drop(lease);
    let copied = FolderProject::open(target.path()).await.unwrap();
    assert_eq!(
        copied.capture().await.unwrap().project().id,
        saved.project().id
    );
}
