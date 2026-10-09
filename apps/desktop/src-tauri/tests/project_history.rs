use forma_core::{
    models::{Parameters, Project, Revision},
    project_history::{self, Direction, HistoryState, PreparedHistory},
    storage,
};
use sqlx::{SqliteConnection, SqlitePool};

fn empty() -> Project {
    Project {
        schema_version: 1,
        id: uuid::Uuid::new_v4().to_string(),
        name: "History fixture".into(),
        units: "mm".into(),
        agent: "codex".into(),
        pinned: false,
        thumbnail: None,
        thumbnail_revision: None,
        created_at: "2026-10-04T12:00:00Z".into(),
        updated_at: "2026-10-04T12:00:00Z".into(),
        revisions: vec![],
        current_revision: None,
        messages: vec![],
        files: vec![],
        exports: vec![],
    }
}

fn edit(current: &Project, thickness: f64, radius: f64) -> Project {
    let mut next = current.clone();
    let id = uuid::Uuid::new_v4().to_string();
    let mut ir: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../docs/fixtures/parameterized-disc.cad.json"
    ))
    .unwrap();
    ir["revisionId"] = id.clone().into();
    ir["parameters"][0]["valueMm"] = thickness.into();
    ir["parameters"][1]["valueMm"] = radius.into();
    next.revisions.push(Revision {
        id: id.clone(),
        parent: current.current_revision.clone(),
        created_at: "2026-10-04T12:01:00Z".into(),
        prompt: "One approved AI/batch edit".into(),
        parameters: Parameters {
            kind: "bracket".into(),
            width: 120.0,
            depth: 65.0,
            height: 60.0,
            thickness: 5.0,
            hole_diameter: 8.0,
            holes: 4,
        },
        source: None,
        preview: None,
        program: Some(ir.to_string()),
        program_base: None,
    });
    next.current_revision = Some(id);
    next.validate().unwrap();
    next
}

async fn upsert(connection: &mut SqliteConnection, project: &Project) {
    sqlx::query("INSERT INTO projects(id,name,payload,updated_at) VALUES(?,?,?,?) ON CONFLICT(id) DO UPDATE SET payload=excluded.payload")
        .bind(&project.id).bind(&project.name).bind(serde_json::to_string(project).unwrap()).bind(&project.updated_at)
        .execute(connection).await.unwrap();
}

async fn commit(
    pool: &SqlitePool,
    old: Option<&Project>,
    new: &Project,
    intent: Option<&PreparedHistory>,
) {
    let mut transaction = pool.begin().await.unwrap();
    upsert(&mut transaction, new).await;
    project_history::record_commit(&mut transaction, old, new, intent)
        .await
        .unwrap();
    transaction.commit().await.unwrap();
}

async fn database() -> (tempfile::TempDir, SqlitePool) {
    let temp = tempfile::tempdir().unwrap();
    let pool = storage::open(&temp.path().join("history.sqlite"))
        .await
        .unwrap();
    (temp, pool)
}

async fn travel(pool: &SqlitePool, project: &Project, direction: Direction) -> Project {
    let state = project_history::load_state(pool, project).await.unwrap();
    let (next, intent) = project_history::prepare(
        project,
        &state,
        direction,
        project.current_revision.as_deref(),
        &uuid::Uuid::new_v4().to_string(),
        "2026-10-04T12:02:00Z",
    )
    .unwrap();
    commit(pool, Some(project), &next, Some(&intent)).await;
    next
}

fn values(project: &Project) -> (f64, f64) {
    let current = project
        .revisions
        .iter()
        .find(|revision| Some(&revision.id) == project.current_revision.as_ref())
        .unwrap();
    let ir: serde_json::Value = serde_json::from_str(current.program.as_ref().unwrap()).unwrap();
    assert_eq!(ir["revisionId"].as_str(), Some(current.id.as_str()));
    (
        ir["parameters"][0]["valueMm"].as_f64().unwrap(),
        ir["parameters"][1]["valueMm"].as_f64().unwrap(),
    )
}

async fn events(pool: &SqlitePool, project: &Project) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM project_history_events WHERE project_id=?")
        .bind(&project.id)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn whole_parameter_batch_is_one_undo_and_redo_step_with_immutable_revisions() {
    let (_temp, pool) = database().await;
    let base = edit(&empty(), 8.0, 4.0);
    commit(&pool, None, &base, None).await;
    let mut updated = edit(&base, 12.0, 6.0);
    let current_ir: forma_core::cad_ir::Document =
        serde_json::from_str(base.revisions[0].program.as_ref().unwrap()).unwrap();
    let commands = [
        forma_core::cad_ir::Command::SetParameter {
            parameter_id: "thickness".into(),
            value_mm: 12.0,
        },
        forma_core::cad_ir::Command::SetParameter {
            parameter_id: "hole_radius".into(),
            value_mm: 6.0,
        },
    ];
    let staged = forma_core::cad_ir::apply_commands(
        &current_ir,
        base.current_revision.as_ref().unwrap(),
        updated.current_revision.as_ref().unwrap(),
        &commands,
    )
    .unwrap();
    updated.revisions.last_mut().unwrap().program = Some(serde_json::to_string(&staged).unwrap());
    commit(&pool, Some(&base), &updated, None).await;
    assert_eq!(events(&pool, &updated).await, 2);
    let undone = travel(&pool, &updated, Direction::Undo).await;
    assert_eq!(values(&undone), (8.0, 4.0));
    assert_eq!(undone.revisions.len(), updated.revisions.len() + 1);
    assert!(undone.revisions.starts_with(&updated.revisions));
    assert_eq!(
        undone.revisions.last().unwrap().parent,
        updated.current_revision
    );
    let redone = travel(&pool, &undone, Direction::Redo).await;
    assert_eq!(values(&redone), (12.0, 6.0));
    assert!(redone.revisions.starts_with(&undone.revisions));
    assert_eq!(events(&pool, &redone).await, 4);
    assert!(!project_history::load_state(&pool, &redone)
        .await
        .unwrap()
        .can_redo());
}

#[tokio::test]
async fn first_creation_undo_keeps_history_and_redo_creates_a_new_saved_snapshot() {
    let (_temp, pool) = database().await;
    let base = edit(&empty(), 8.0, 4.0);
    commit(&pool, None, &base, None).await;
    let undone = travel(&pool, &base, Direction::Undo).await;
    assert!(undone.current_revision.is_none());
    assert_eq!(undone.revisions, base.revisions);
    let state = project_history::load_state(&pool, &undone).await.unwrap();
    assert!(!state.can_undo());
    assert!(state.can_redo());
    let redone = travel(&pool, &undone, Direction::Redo).await;
    assert_eq!(values(&redone), (8.0, 4.0));
    assert_ne!(redone.current_revision, base.current_revision);
    assert!(redone.revisions.last().unwrap().parent.is_none());
}

#[tokio::test]
async fn new_edit_after_undo_discards_redo_cursor_without_deleting_old_snapshots() {
    let (_temp, pool) = database().await;
    let base = edit(&empty(), 8.0, 4.0);
    commit(&pool, None, &base, None).await;
    let second = edit(&base, 12.0, 6.0);
    commit(&pool, Some(&base), &second, None).await;
    let undone = travel(&pool, &second, Direction::Undo).await;
    let branch = edit(&undone, 10.0, 5.0);
    commit(&pool, Some(&undone), &branch, None).await;
    assert!(branch.revisions.starts_with(&second.revisions));
    let state = project_history::load_state(&pool, &branch).await.unwrap();
    assert!(!state.can_redo());
    let returned = travel(&pool, &branch, Direction::Undo).await;
    assert_eq!(values(&returned), (8.0, 4.0));
    assert_eq!(
        values(&travel(&pool, &returned, Direction::Redo).await),
        (10.0, 5.0)
    );
}

#[tokio::test]
async fn chat_and_metadata_saves_preserve_redo_and_create_no_model_events() {
    let (_temp, pool) = database().await;
    let base = edit(&empty(), 8.0, 4.0);
    commit(&pool, None, &base, None).await;
    let undone = travel(&pool, &base, Direction::Undo).await;
    let before = project_history::load_state(&pool, &undone).await.unwrap();
    let mut metadata = undone.clone();
    metadata.name = "Renamed".into();
    metadata.pinned = true;
    metadata.messages.push(forma_core::models::Message {
        id: "message".into(),
        role: "assistant".into(),
        text: "Review only".into(),
        created_at: "now".into(),
    });
    commit(&pool, Some(&undone), &metadata, None).await;
    assert_eq!(
        before,
        project_history::load_state(&pool, &metadata).await.unwrap()
    );
    assert_eq!(events(&pool, &metadata).await, 2);
}

#[tokio::test]
async fn stale_and_empty_actions_leave_original_project_and_database_unchanged() {
    let (_temp, pool) = database().await;
    let base = edit(&empty(), 8.0, 4.0);
    commit(&pool, None, &base, None).await;
    let state = project_history::load_state(&pool, &base).await.unwrap();
    let before = serde_json::to_string(&base).unwrap();
    assert!(project_history::prepare(
        &base,
        &state,
        Direction::Undo,
        None,
        &uuid::Uuid::new_v4().to_string(),
        "now"
    )
    .is_err());
    assert!(project_history::prepare(
        &base,
        &state,
        Direction::Redo,
        base.current_revision.as_deref(),
        &uuid::Uuid::new_v4().to_string(),
        "now"
    )
    .is_err());
    assert_eq!(before, serde_json::to_string(&base).unwrap());
    assert_eq!(
        state,
        project_history::load_state(&pool, &base).await.unwrap()
    );
    assert_eq!(events(&pool, &base).await, 1);
}

#[tokio::test]
async fn rollback_after_history_write_restores_project_cursor_and_event_journal_together() {
    let (_temp, pool) = database().await;
    let base = edit(&empty(), 8.0, 4.0);
    commit(&pool, None, &base, None).await;
    let state = project_history::load_state(&pool, &base).await.unwrap();
    let (next, intent) = project_history::prepare(
        &base,
        &state,
        Direction::Undo,
        base.current_revision.as_deref(),
        &uuid::Uuid::new_v4().to_string(),
        "now",
    )
    .unwrap();
    let mut tx = pool.begin().await.unwrap();
    upsert(&mut tx, &next).await;
    project_history::record_commit(&mut tx, Some(&base), &next, Some(&intent))
        .await
        .unwrap();
    tx.rollback().await.unwrap();
    let raw: String = sqlx::query_scalar("SELECT payload FROM projects WHERE id=?")
        .bind(&base.id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(raw, serde_json::to_string(&base).unwrap());
    assert_eq!(
        state,
        project_history::load_state(&pool, &base).await.unwrap()
    );
    assert_eq!(events(&pool, &base).await, 1);
}

#[tokio::test]
async fn modified_prepared_snapshot_and_old_revision_overwrite_are_rejected_atomically() {
    let (_temp, pool) = database().await;
    let base = edit(&empty(), 8.0, 4.0);
    commit(&pool, None, &base, None).await;
    let state = project_history::load_state(&pool, &base).await.unwrap();
    let (mut next, intent) = project_history::prepare(
        &base,
        &state,
        Direction::Undo,
        base.current_revision.as_deref(),
        &uuid::Uuid::new_v4().to_string(),
        "now",
    )
    .unwrap();
    next.name = "Tampered after preparation".into();
    let mut tx = pool.begin().await.unwrap();
    upsert(&mut tx, &next).await;
    assert!(
        project_history::record_commit(&mut tx, Some(&base), &next, Some(&intent))
            .await
            .is_err()
    );
    tx.rollback().await.unwrap();
    let mut forged = edit(&base, 12.0, 6.0);
    forged.revisions[0].prompt = "Overwrite historical data".into();
    let mut tx = pool.begin().await.unwrap();
    assert!(
        project_history::record_commit(&mut tx, Some(&base), &forged, None)
            .await
            .is_err()
    );
    tx.rollback().await.unwrap();
    assert_eq!(
        state,
        project_history::load_state(&pool, &base).await.unwrap()
    );
    assert_eq!(events(&pool, &base).await, 1);
}

#[tokio::test]
async fn portable_copy_with_a_new_project_uuid_preserves_redo_and_rejects_cursor_tampering() {
    let (_temp, pool) = database().await;
    let base = edit(&empty(), 8.0, 4.0);
    commit(&pool, None, &base, None).await;
    let undone = travel(&pool, &base, Direction::Undo).await;
    let archive = project_history::export_history(&mut pool.acquire().await.unwrap(), &undone)
        .await
        .unwrap();
    let mut copied = undone.clone();
    copied.id = uuid::Uuid::new_v4().to_string();
    let mut tx = pool.begin().await.unwrap();
    upsert(&mut tx, &copied).await;
    project_history::import_history(&mut tx, &copied, &archive)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert!(project_history::load_state(&pool, &copied)
        .await
        .unwrap()
        .can_redo());
    assert_eq!(
        values(&travel(&pool, &copied, Direction::Redo).await),
        (8.0, 4.0)
    );
    let mut bad: serde_json::Value = serde_json::from_slice(&archive).unwrap();
    bad["state"]["cursor"] = 1.into();
    let mut rejected = undone.clone();
    rejected.id = uuid::Uuid::new_v4().to_string();
    let mut tx = pool.begin().await.unwrap();
    upsert(&mut tx, &rejected).await;
    assert!(
        project_history::import_history(&mut tx, &rejected, bad.to_string().as_bytes())
            .await
            .is_err()
    );
    tx.rollback().await.unwrap();
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM projects WHERE id=?")
        .bind(&rejected.id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn legacy_parent_chain_ignores_abandoned_siblings_and_history_does_not_need_native_worker() {
    let first = edit(&empty(), 8.0, 4.0);
    let second = edit(&first, 12.0, 6.0);
    let mut fork = edit(&second, 10.0, 5.0);
    fork.revisions.last_mut().unwrap().parent = first.current_revision.clone();
    let state = HistoryState::from_project(&fork).unwrap();
    let (previous, _) = project_history::prepare(
        &fork,
        &state,
        Direction::Undo,
        fork.current_revision.as_deref(),
        &uuid::Uuid::new_v4().to_string(),
        "now",
    )
    .unwrap();
    assert_eq!(values(&previous), (8.0, 4.0));
    assert!(!state.can_redo());
}

#[tokio::test]
async fn ordinary_restore_and_a_history_like_prompt_remain_new_undoable_model_actions() {
    let (_temp, pool) = database().await;
    let first = edit(&empty(), 8.0, 4.0);
    commit(&pool, None, &first, None).await;
    let second = edit(&first, 12.0, 6.0);
    commit(&pool, Some(&first), &second, None).await;
    let mut restored = edit(&second, 8.0, 4.0);
    restored.revisions.last_mut().unwrap().prompt =
        format!("history:undo:{}", first.current_revision.as_ref().unwrap());
    commit(&pool, Some(&second), &restored, None).await;
    let previous = travel(&pool, &restored, Direction::Undo).await;
    assert_eq!(values(&previous), (12.0, 6.0));
    assert_eq!(
        values(&travel(&pool, &previous, Direction::Redo).await),
        (8.0, 4.0)
    );
    let kinds: Vec<String> = sqlx::query_scalar(
        "SELECT kind FROM project_history_events WHERE project_id=? ORDER BY id",
    )
    .bind(&first.id)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(kinds, ["edit", "edit", "edit", "undo", "redo"]);
}

#[tokio::test]
async fn saved_undo_cursor_survives_database_reopen_and_invalid_archive_ids_are_rejected() {
    let (temp, pool) = database().await;
    let first = edit(&empty(), 8.0, 4.0);
    commit(&pool, None, &first, None).await;
    let second = edit(&first, 12.0, 6.0);
    commit(&pool, Some(&first), &second, None).await;
    let undone = travel(&pool, &second, Direction::Undo).await;
    let original_state = project_history::load_state(&pool, &undone).await.unwrap();
    let archive = project_history::export_history(&mut pool.acquire().await.unwrap(), &undone)
        .await
        .unwrap();
    pool.close().await;
    let pool = storage::open(&temp.path().join("history.sqlite"))
        .await
        .unwrap();
    assert_eq!(
        original_state,
        project_history::load_state(&pool, &undone).await.unwrap()
    );
    assert!(original_state.can_redo());
    for variant in 0..5 {
        let mut value: serde_json::Value = serde_json::from_slice(&archive).unwrap();
        match variant {
            0 => value["schemaVersion"] = 2.into(),
            1 => value["events"][0]["targetRevision"] = uuid::Uuid::new_v4().to_string().into(),
            2 => value["events"][1]["fromRevision"] = serde_json::Value::Null,
            // Existing IDs alone do not prove a truthful undo snapshot.
            3 => {
                value["events"][2]["targetRevision"] =
                    second.current_revision.clone().unwrap().into()
            }
            _ => {
                value["events"][2]["toRevision"] = first.current_revision.clone().unwrap().into();
                value["state"]["head"] = first.current_revision.clone().unwrap().into();
            }
        }
        let mut copy = undone.clone();
        copy.id = uuid::Uuid::new_v4().to_string();
        let mut tx = pool.begin().await.unwrap();
        upsert(&mut tx, &copy).await;
        assert!(
            project_history::import_history(&mut tx, &copy, value.to_string().as_bytes())
                .await
                .is_err()
        );
        tx.rollback().await.unwrap();
    }
    assert_eq!(
        original_state,
        project_history::load_state(&pool, &undone).await.unwrap()
    );
}

#[tokio::test]
async fn imported_legacy_baseline_with_parent_chain_remains_portable() {
    let (_temp, pool) = database().await;
    let first = edit(&empty(), 8.0, 4.0);
    let second = edit(&first, 12.0, 6.0);
    // Old bundles can already contain several revisions before the cursor exists.
    commit(&pool, None, &second, None).await;
    let state = project_history::load_state(&pool, &second).await.unwrap();
    assert!(state.can_undo());
    let archive = project_history::export_history(&mut pool.acquire().await.unwrap(), &second)
        .await
        .unwrap();
    project_history::validate_archive(&second, &archive).unwrap();
    let third = edit(&second, 16.0, 7.0);
    commit(&pool, Some(&second), &third, None).await;
    let archive = project_history::export_history(&mut pool.acquire().await.unwrap(), &third)
        .await
        .unwrap();
    project_history::validate_archive(&third, &archive).unwrap();
    let undone = travel(&pool, &third, Direction::Undo).await;
    let archive = project_history::export_history(&mut pool.acquire().await.unwrap(), &undone)
        .await
        .unwrap();
    project_history::validate_archive(&undone, &archive).unwrap();
    let mut wrong_parent = undone.clone();
    wrong_parent.revisions.last_mut().unwrap().parent = first.current_revision;
    assert!(project_history::validate_archive(&wrong_parent, &archive).is_err());
}
