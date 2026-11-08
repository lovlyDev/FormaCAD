use super::prepare_commit;
use crate::{
    models::Project,
    project_history::{prepare, Direction, HistoryState},
};

fn project(revisions: usize) -> Project {
    let mut project: Project = serde_json::from_value(serde_json::json!({
        "schemaVersion":1,"id":uuid::Uuid::new_v4().to_string(),"name":"History transition fixture",
        "units":"mm","agent":"codex","pinned":false,
        "createdAt":"2026-10-29T00:00:00Z","updatedAt":"2026-10-29T00:00:00Z",
        "currentRevision":null,"messages":[],"exports":[],"files":[],"revisions":[]
    }))
    .unwrap();
    for _ in 0..revisions {
        append(&mut project);
    }
    project.validate().unwrap();
    project
}

fn append(project: &mut Project) {
    let id = uuid::Uuid::new_v4().to_string();
    project.revisions.push(
        serde_json::from_value(serde_json::json!({
            "id":id,"parent":project.current_revision,"createdAt":"2026-10-29T00:00:00Z",
            "prompt":"Synthetic model edit",
            "parameters":{"kind":"box","width":40+project.revisions.len(),"depth":20,"height":10,
                "thickness":2,"holeDiameter":0,"holes":0}
        }))
        .unwrap(),
    );
    project.current_revision = Some(id);
}

fn code<T>(result: crate::core::Result<T>) -> String {
    result
        .err()
        .expect("Expected rejected transition")
        .to_string()
}

#[test]
fn initial_commit_retains_parent_chain_and_records_only_current_head() {
    let new = project(2);
    let prepared = prepare_commit(None, None, &new, None).unwrap();
    assert_eq!(prepared.state(), &HistoryState::from_project(&new).unwrap());
    assert_eq!(
        prepared.event(),
        Some(("edit", new.current_revision.as_deref()))
    );
    let empty = project(0);
    assert!(prepare_commit(None, None, &empty, None)
        .unwrap()
        .event()
        .is_none());
}

#[test]
fn metadata_only_change_retains_cursor_and_creates_no_event() {
    let old = project(2);
    let persisted = HistoryState::from_project(&old).unwrap();
    let before = serde_json::to_vec(&old).unwrap();
    let mut new = old.clone();
    new.name = "Updated name".into();
    let prepared = prepare_commit(Some(&old), Some(&persisted), &new, None).unwrap();
    assert_eq!(prepared.state(), &persisted);
    assert!(prepared.event().is_none());
    assert_eq!(serde_json::to_vec(&old).unwrap(), before);
}

#[test]
fn appended_model_revision_advances_history_once() {
    let old = project(1);
    let persisted = HistoryState::from_project(&old).unwrap();
    let mut new = old.clone();
    append(&mut new);
    let prepared = prepare_commit(Some(&old), Some(&persisted), &new, None).unwrap();
    assert!(prepared.state().can_undo());
    assert!(!prepared.state().can_redo());
    assert_eq!(
        prepared.event(),
        Some(("edit", new.current_revision.as_deref()))
    );
    let json = serde_json::to_value(prepared.state()).unwrap();
    assert_eq!(json["generation"], 1);
    assert_eq!(json["cursor"], 2);
    assert_eq!(
        json["head"],
        serde_json::to_value(&new.current_revision).unwrap()
    );
}

#[test]
fn existing_revision_rewrite_is_rejected_even_when_project_validates() {
    let old = project(1);
    let mut new = old.clone();
    new.revisions[0].prompt = "Rewritten immutable revision".into();
    new.validate().unwrap();
    assert_eq!(
        code(prepare_commit(Some(&old), None, &new, None)),
        "HISTORY_IMMUTABLE"
    );
}

#[test]
fn added_revision_without_moving_head_is_rejected() {
    let old = project(1);
    let mut new = old.clone();
    append(&mut new);
    new.current_revision = old.current_revision.clone();
    new.validate().unwrap();
    assert_eq!(
        code(prepare_commit(Some(&old), None, &new, None)),
        "HISTORY_INVALID_STATE"
    );
}

#[test]
fn undo_checks_original_project_candidate_and_persisted_cursor() {
    let old = project(2);
    let persisted = HistoryState::from_project(&old).unwrap();
    let (new, transition) = prepare(
        &old,
        &persisted,
        Direction::Undo,
        old.current_revision.as_deref(),
        &uuid::Uuid::new_v4().to_string(),
        "2026-10-29T01:00:00Z",
    )
    .unwrap();
    let prepared = prepare_commit(Some(&old), Some(&persisted), &new, Some(&transition)).unwrap();
    assert!(prepared.state().can_redo());
    assert_eq!(
        prepared.event(),
        Some(("undo", Some(old.revisions[0].id.as_str())))
    );

    // This cursor is independently valid for the same old model, but belongs
    // to a different persisted generation. It must not authorize the intent.
    let mut changed = serde_json::to_value(&persisted).unwrap();
    changed["generation"] = serde_json::json!(1);
    let changed: HistoryState = serde_json::from_value(changed).unwrap();
    assert_eq!(
        code(prepare_commit(
            Some(&old),
            Some(&changed),
            &new,
            Some(&transition)
        )),
        "Project changed; retry the CAD edit"
    );
    let mut changed_candidate = new.clone();
    changed_candidate.name = "Changed after preparation".into();
    assert_eq!(
        code(prepare_commit(
            Some(&old),
            Some(&persisted),
            &changed_candidate,
            Some(&transition)
        )),
        "Project changed; retry the CAD edit"
    );
    assert_eq!(
        code(prepare_commit(None, None, &new, Some(&transition))),
        "HISTORY_INVALID_STATE"
    );
}

#[test]
fn absent_legacy_cursor_matches_explicit_parent_chain_state() {
    let old = project(2);
    let persisted = HistoryState::from_project(&old).unwrap();
    let mut new = old.clone();
    append(&mut new);
    let fallback = prepare_commit(Some(&old), None, &new, None).unwrap();
    let explicit = prepare_commit(Some(&old), Some(&persisted), &new, None).unwrap();
    assert_eq!(fallback.state(), explicit.state());
    assert_eq!(fallback.event(), explicit.event());
}

#[tokio::test]
async fn sql_writer_records_the_pure_transition_inside_the_callers_transaction() {
    use crate::project_repository::legacy::read_test_fixture as fixture;
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let old = fixture::project(&state, 16).await;
    let before = fixture::logical(&state, &old.id).await;
    let persisted = crate::project_history::load_state(&state.pool, &old)
        .await
        .unwrap();
    let mut new = old.clone();
    append(&mut new);
    let expected = prepare_commit(Some(&old), Some(&persisted), &new, None).unwrap();
    let mut transaction = state.pool.begin().await.unwrap();
    sqlx::query("UPDATE projects SET payload=? WHERE id=?")
        .bind(serde_json::to_string(&new).unwrap())
        .bind(&new.id)
        .execute(&mut *transaction)
        .await
        .unwrap();
    crate::project_history::record_commit(&mut transaction, Some(&old), &new, None)
        .await
        .unwrap();
    let stored: String =
        sqlx::query_scalar("SELECT state_json FROM project_history WHERE project_id=?")
            .bind(&new.id)
            .fetch_one(&mut *transaction)
            .await
            .unwrap();
    assert_eq!(stored, serde_json::to_string(expected.state()).unwrap());
    let event: (String, Option<String>, Option<String>, Option<String>, String) =
        sqlx::query_as("SELECT kind,from_revision,to_revision,target_revision,created_at FROM project_history_events WHERE project_id=? ORDER BY id DESC LIMIT 1")
            .bind(&new.id).fetch_one(&mut *transaction).await.unwrap();
    assert_eq!(
        event,
        (
            "edit".into(),
            old.current_revision.clone(),
            new.current_revision.clone(),
            new.current_revision.clone(),
            new.updated_at.clone()
        )
    );
    let archive = crate::project_history::export_history(&mut transaction, &new)
        .await
        .unwrap();
    crate::project_history::validate_archive(&new, &archive).unwrap();
    transaction.rollback().await.unwrap();
    assert_eq!(fixture::logical(&state, &old.id).await, before);
}
