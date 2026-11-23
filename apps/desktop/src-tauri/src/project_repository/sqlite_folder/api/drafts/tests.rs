use super::*;
use crate::{models::Project, project_repository::sqlite_folder::api::Publication};

fn project() -> Project {
    let revision = Uuid::new_v4().to_string();
    serde_json::from_value(serde_json::json!({
        "schemaVersion":1,"id":Uuid::new_v4().to_string(),"name":"Draft provenance fixture",
        "units":"mm","agent":"codex","pinned":false,"createdAt":"2026-11-01T00:00:00Z",
        "updatedAt":"2026-11-01T00:00:00Z","currentRevision":revision,"messages":[],
        "exports":[],"files":[],"revisions":[{
            "id":revision,"parent":null,"createdAt":"2026-11-01T00:00:00Z","prompt":"Initial",
            "parameters":{"kind":"box","width":40,"depth":20,"height":10,"thickness":1,"holeDiameter":0,"holes":0}
        }]
    }))
    .unwrap()
}

async fn create(root: &Path, project: Project) -> FolderProject {
    assert!(matches!(
        FolderProject::create(root, project, vec![], None)
            .await
            .unwrap(),
        Publication::Committed(_)
    ));
    FolderProject::open(root).await.unwrap()
}

#[tokio::test]
async fn explicit_draft_survives_close_without_modifying_the_saved_project() {
    let selected = tempfile::tempdir().unwrap();
    let local = tempfile::tempdir().unwrap();
    let store = LocalDraftStore::open(local.path()).unwrap();
    let source = create(selected.path(), project()).await;
    let original = source.capture().await.unwrap();
    let before = fs::read(selected.path().join("project.sqlite")).unwrap();
    let approval = source.approve_local_draft(&original).await.unwrap();
    source.close();
    assert!(source.approve_local_draft(&original).await.is_err());

    let unfinished = "{ \"schemaVersion\": 2, \"features\": [";
    let saved = store.save(approval, unfinished.into()).unwrap();
    let serialized = fs::read_to_string(store.path(saved.id()).unwrap()).unwrap();
    for token in ["sourceId", "openingId", "approvalId", "taskScopeId"] {
        assert!(!serialized.contains(token));
    }

    let reopened = FolderProject::open(selected.path()).await.unwrap();
    let viewed = reopened.capture().await.unwrap();
    let draft = reopened
        .read_local_draft(&viewed, &store, saved.id())
        .await
        .unwrap();
    assert_eq!(draft.editor_text(), unfinished);
    assert_eq!(draft.provenance().project_id(), original.project().id);
    assert_eq!(draft.provenance().base_commit_id(), original.commit_id());
    assert_eq!(
        draft.provenance().base_revision_id(),
        original.project().current_revision.as_deref()
    );
    assert_eq!(
        fs::read(selected.path().join("project.sqlite")).unwrap(),
        before
    );
}

#[tokio::test]
async fn draft_cannot_be_restored_to_another_repository_or_newer_commit() {
    let selected = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    let local = tempfile::tempdir().unwrap();
    let store = LocalDraftStore::open(local.path()).unwrap();
    let initial = project();
    let source = create(selected.path(), initial.clone()).await;
    let original = source.capture().await.unwrap();
    let approval = source.approve_local_draft(&original).await.unwrap();
    let saved = store.save(approval, "unchanged draft".into()).unwrap();

    let replacement = create(other.path(), initial).await;
    let view = replacement.capture().await.unwrap();
    assert_eq!(view.project().id, original.project().id);
    assert!(replacement
        .read_local_draft(&view, &store, saved.id())
        .await
        .is_err());

    let approval = source.approve_write(&original).await.unwrap();
    let mut renamed = original.project().clone();
    renamed.name = "External metadata became current".into();
    assert!(matches!(
        source
            .commit(&original, approval, renamed, vec![])
            .await
            .unwrap(),
        Publication::Committed(_)
    ));
    let latest = source.capture().await.unwrap();
    assert_eq!(
        latest.project().current_revision,
        original.project().current_revision
    );
    assert!(source
        .read_local_draft(&latest, &store, saved.id())
        .await
        .is_err());
}

#[tokio::test]
async fn oversized_draft_leaves_no_files() {
    let selected = tempfile::tempdir().unwrap();
    let local = tempfile::tempdir().unwrap();
    let store = LocalDraftStore::open(local.path()).unwrap();
    let source = create(selected.path(), project()).await;
    let original = source.capture().await.unwrap();
    let approval = source.approve_local_draft(&original).await.unwrap();
    assert!(store
        .save(approval, "x".repeat(MAX_DRAFT_BYTES + 1))
        .is_err());
    assert_eq!(fs::read_dir(&store.root).unwrap().count(), 0);
}
