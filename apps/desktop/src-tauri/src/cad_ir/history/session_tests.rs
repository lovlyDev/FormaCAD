use super::{compare_captured, compare_saved};
use crate::{
    core::AppState,
    models::Project,
    project_access::AccessRegistry,
    project_repository::{backend::ProjectRepository, legacy::read_test_fixture as fixture},
};
use std::{future::Future, task::Poll};

fn document(id: &str, width: f64) -> String {
    serde_json::json!({"schemaVersion":2,"revisionId":id,"parameters":[],"features":[
        {"id":"profile","name":"Profile","operation":{"type":"rectangle",
            "width":{"kind":"literal","mm":width},"depth":{"kind":"literal","mm":20}}},
        {"id":"pad","name":"Pad","operation":{"type":"extrude","sketchId":"profile",
            "distance":{"kind":"literal","mm":10}}}
    ],"bodies":[{"id":"body","name":"Body","sourceFeatureId":"pad"}]})
    .to_string()
}

async fn project(state: &AppState, authored: bool) -> Project {
    let first = uuid::Uuid::new_v4().to_string();
    let second = uuid::Uuid::new_v4().to_string();
    let revision = |id: &str, parent: Option<&str>, width: f64| {
        serde_json::json!({"id":id,"parent":parent,"createdAt":"2026-10-31T00:00:00Z",
            "prompt":"Synthetic structural revision",
            "program":authored.then(|| document(id, width)),
            "parameters":{"kind":"box","width":width,"depth":20,"height":10,
                "thickness":2,"holeDiameter":0,"holes":0}})
    };
    let project: Project = serde_json::from_value(serde_json::json!({
        "schemaVersion":1,"id":uuid::Uuid::new_v4().to_string(),"name":"Revision diff fixture",
        "units":"mm","agent":"codex","pinned":false,
        "createdAt":"2026-10-31T00:00:00Z","updatedAt":"2026-10-31T00:00:00Z",
        "currentRevision":second,"messages":[],"files":[],"exports":[],
        "revisions":[revision(&first,None,40.),revision(&second,Some(&first),60.)]
    }))
    .unwrap();
    fixture::replace(state, &project, true).await;
    let root = state.root.join(&project.id);
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("sentinel.txt"), b"unchanged legacy source").unwrap();
    project
}

#[tokio::test]
async fn saved_diff_uses_both_original_revisions_without_source_writes() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let project = project(&state, true).await;
    let before = fixture::logical(&state, &project.id).await;
    let files = fixture::files(&state.root.join(&project.id));
    let diff = compare_saved(
        &state,
        &project.id,
        &project.revisions[0].id,
        &project.revisions[1].id,
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(diff.from_revision_id, project.revisions[0].id);
    assert_eq!(diff.to_revision_id, project.revisions[1].id);
    assert_eq!(diff.features.len(), 1);
    assert_eq!(
        diff.features[0].before.as_ref().unwrap().value.id,
        "profile"
    );
    assert_eq!(diff.features[0].after.as_ref().unwrap().value.id, "profile");
    assert_eq!(fixture::logical(&state, &project.id).await, before);
    assert_eq!(fixture::files(&state.root.join(&project.id)), files);
    assert_eq!(
        state
            .project_access
            .status(&state.root, &project.id)
            .unwrap()
            .mode,
        "closed"
    );
}

#[tokio::test]
async fn close_and_reopen_during_final_sql_recheck_rejects_computed_diff() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let project = project(&state, true).await;
    let repository = ProjectRepository::managed(&state, &project.id).unwrap();
    let snapshot = repository.capture().await.unwrap();
    let before = fixture::logical(&state, &project.id).await;
    let files = fixture::files(&state.root.join(&project.id));
    let mut held = Vec::new();
    for _ in 0..state.pool.options().get_max_connections() {
        held.push(state.pool.acquire().await.unwrap());
    }
    let comparing = compare_captured(
        &repository,
        &snapshot,
        &project.revisions[0].id,
        &project.revisions[1].id,
    );
    tokio::pin!(comparing);
    // Computing the structural diff is synchronous; pending here is the final
    // original-source SQL recheck, blocked by actual held pool connections.
    std::future::poll_fn(|context| {
        assert!(comparing.as_mut().poll(context).is_pending());
        Poll::Ready(())
    })
    .await;
    state.project_access.release(&project.id).unwrap();
    state
        .project_access
        .acquire(&state.root, &project.id)
        .unwrap();
    drop(held);
    assert_eq!(
        comparing.await.err().unwrap().to_string(),
        "PROJECT_SNAPSHOT_CHANGED"
    );
    assert_eq!(fixture::logical(&state, &project.id).await, before);
    assert_eq!(fixture::files(&state.root.join(&project.id)), files);
    let fresh = compare_saved(
        &state,
        &project.id,
        &project.revisions[0].id,
        &project.revisions[1].id,
    )
    .await
    .unwrap();
    assert!(fresh.is_some());
    state.project_access.release(&project.id).unwrap();
}

#[tokio::test]
async fn pending_recovery_is_refused_without_parsing_or_mutating_its_journal() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let project = project(&state, true).await;
    fixture::pending(&state, &project.id).await;
    let before = fixture::logical(&state, &project.id).await;
    let files = fixture::files(&state.root.join(&project.id));
    let journal: String = sqlx::query_scalar(
        "SELECT target_manifest FROM project_storage_commits WHERE project_id=?",
    )
    .bind(&project.id)
    .fetch_one(&state.pool)
    .await
    .unwrap();
    let error = compare_saved(
        &state,
        &project.id,
        &project.revisions[0].id,
        &project.revisions[1].id,
    )
    .await
    .err()
    .unwrap();
    assert_eq!(error.to_string(), "PROJECT_SNAPSHOT_RECOVERY_REQUIRED");
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT target_manifest FROM project_storage_commits WHERE project_id=?"
        )
        .bind(&project.id)
        .fetch_one(&state.pool)
        .await
        .unwrap(),
        journal
    );
    assert_eq!(fixture::logical(&state, &project.id).await, before);
    assert_eq!(fixture::files(&state.root.join(&project.id)), files);
}

#[tokio::test]
async fn identical_project_and_revision_ids_at_another_root_do_not_authorize_the_snapshot() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let state = fixture::state(first.path()).await;
    let other = fixture::state(second.path()).await;
    let project = project(&state, true).await;
    fixture::replace(&other, &project, true).await;
    let original = ProjectRepository::managed(&state, &project.id).unwrap();
    let snapshot = original.capture().await.unwrap();
    let foreign = ProjectRepository::managed(&other, &project.id).unwrap();
    let before = fixture::logical(&other, &project.id).await;
    let error = compare_captured(
        &foreign,
        &snapshot,
        &project.revisions[0].id,
        &project.revisions[1].id,
    )
    .await
    .err()
    .unwrap();
    assert_eq!(error.to_string(), "PROJECT_SNAPSHOT_CHANGED");
    assert_eq!(fixture::logical(&other, &project.id).await, before);
    original.recheck(&snapshot).await.unwrap();
}

#[tokio::test]
async fn readonly_legacy_none_remains_pure_and_never_upgrades_source_access() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let project = project(&state, false).await;
    let owner = AccessRegistry::default();
    owner.acquire(&state.root, &project.id).unwrap();
    assert_eq!(
        state
            .project_access
            .acquire(&state.root, &project.id)
            .unwrap()
            .mode,
        "read_only"
    );
    let before = fixture::logical(&state, &project.id).await;
    let files = fixture::files(&state.root.join(&project.id));
    let result = compare_saved(
        &state,
        &project.id,
        &project.revisions[0].id,
        &project.revisions[1].id,
    )
    .await
    .unwrap();
    assert!(result.is_none());
    assert_eq!(
        state
            .project_access
            .status(&state.root, &project.id)
            .unwrap()
            .mode,
        "read_only"
    );
    assert!(state
        .project_access
        .ensure_write(&state.root, &project.id)
        .is_err());
    owner.release(&project.id).unwrap();
    assert!(state
        .project_access
        .ensure_write(&state.root, &project.id)
        .is_err());
    assert_eq!(fixture::logical(&state, &project.id).await, before);
    assert_eq!(fixture::files(&state.root.join(&project.id)), files);
}

#[tokio::test]
async fn legacy_scripts_and_json_without_schema_keep_none_and_existing_revision_errors() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let mut project = project(&state, false).await;
    for source in ["print('legacy')", "{\"legacy\":true}"] {
        for revision in &mut project.revisions {
            revision.program = Some(source.into());
        }
        fixture::replace(&state, &project, false).await;
        assert!(compare_saved(
            &state,
            &project.id,
            &project.revisions[0].id,
            &project.revisions[1].id
        )
        .await
        .unwrap()
        .is_none());
    }
    let missing = uuid::Uuid::new_v4().to_string();
    assert_eq!(
        compare_saved(&state, &project.id, &missing, &project.revisions[1].id)
            .await
            .err()
            .unwrap()
            .to_string(),
        "CAD revision is missing"
    );
    assert_eq!(
        compare_saved(&state, &missing, &missing, &missing)
            .await
            .err()
            .unwrap()
            .to_string(),
        "Project was not found"
    );
    project.revisions[0].program = Some(document(&missing, 40.));
    fixture::replace(&state, &project, false).await;
    assert_eq!(
        compare_saved(
            &state,
            &project.id,
            &project.revisions[0].id,
            &project.revisions[1].id
        )
        .await
        .err()
        .unwrap()
        .to_string(),
        "CAD IR revision does not match project"
    );
}
