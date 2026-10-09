use super::*;
use serde_json::json;

async fn fixture(root: &std::path::Path) -> (AppState, Project) {
    let pool = crate::storage::open(&root.join("legacy.sqlite"))
        .await
        .unwrap();
    let (_, guard) = tracing_appender::non_blocking(std::io::sink());
    let state = AppState {
        root: root.into(),
        pool,
        _log_guard: guard,
        cad_tasks: Default::default(),
        project_access: Default::default(),
        grants: Default::default(),
        tasks: Default::default(),
        writes: Default::default(),
    };
    let project: Project = serde_json::from_value(
        json!({"schemaVersion":1,"id":uuid::Uuid::new_v4().to_string(),
        "name":"Legacy embedded","units":"mm","agent":"codex","pinned":false,
        "createdAt":"2026-10-08T00:00:00Z","updatedAt":"2026-10-08T00:00:00Z",
        "revisions":[],"currentRevision":null,"messages":[],"files":[],"exports":[]}),
    )
    .unwrap();
    (state, project)
}
async fn insert(state: &AppState, project: &Project) {
    sqlx::query("INSERT INTO projects(id,name,payload,updated_at) VALUES(?,?,?,?)")
        .bind(&project.id)
        .bind(&project.name)
        .bind(serde_json::to_string(project).unwrap())
        .bind(&project.updated_at)
        .execute(&state.pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn real_embedded_legacy_project_over_16_mib_remains_readable_without_writes() {
    let temp = tempfile::tempdir().unwrap();
    let (state, mut project) = fixture(temp.path()).await;
    let bytes = vec![42u8; 20 * 1024 * 1024];
    project.files.push(crate::models::ProjectFile {
        name: "legacy.step".into(),
        kind: "source".into(),
        size: bytes.len() as u64,
        data: Some(crate::artifacts::data_url("legacy.step", &bytes)),
        sha256: None,
    });
    project.validate().unwrap();
    insert(&state, &project).await;
    let raw = serde_json::to_string(&project).unwrap();
    assert!(raw.len() > 16 * 1024 * 1024);
    let captured = capture_project(&state, &project.id).await.unwrap();
    assert_eq!(serde_json::to_string(&captured.project).unwrap(), raw);
    recheck(&state, &captured).await.unwrap();
    assert!(!state.root.join(&project.id).exists());
    let after: String = sqlx::query_scalar("SELECT payload FROM projects WHERE id=?")
        .bind(&project.id)
        .fetch_one(&state.pool)
        .await
        .unwrap();
    assert_eq!(after, raw);
}

#[tokio::test]
async fn caller_mutation_and_raw_history_whitespace_invalidate_full_recheck() {
    let temp = tempfile::tempdir().unwrap();
    let (state, project) = fixture(temp.path()).await;
    insert(&state, &project).await;
    let mut captured = capture_project(&state, &project.id).await.unwrap();
    captured.project.name.push('!');
    assert_eq!(
        recheck(&state, &captured).await.err().unwrap().to_string(),
        "PROJECT_SNAPSHOT_CHANGED"
    );
    let captured = capture_with_history(&state, &project.id).await.unwrap();
    let archive: serde_json::Value =
        serde_json::from_slice(captured.history.as_ref().unwrap()).unwrap();
    sqlx::query("INSERT INTO project_history(project_id,state_json) VALUES(?,?)")
        .bind(&project.id)
        .bind(serde_json::to_string(&archive["state"]).unwrap())
        .execute(&state.pool)
        .await
        .unwrap();
    let captured = capture_with_history(&state, &project.id).await.unwrap();
    sqlx::query("UPDATE project_history SET state_json=state_json || ' ' WHERE project_id=?")
        .bind(&project.id)
        .execute(&state.pool)
        .await
        .unwrap();
    assert_eq!(
        recheck(&state, &captured).await.err().unwrap().to_string(),
        "PROJECT_SNAPSHOT_CHANGED"
    );
}

#[tokio::test]
async fn oversized_journal_is_refused_and_oversized_history_cells_are_bounded() {
    let temp = tempfile::tempdir().unwrap();
    let (state, project) = fixture(temp.path()).await;
    insert(&state, &project).await;
    sqlx::query("INSERT INTO project_storage_commits(project_id,base_manifest_sha256,target_manifest,project_sha256) VALUES(?,NULL,CAST(zeroblob(20000000) AS TEXT),'hash')")
        .bind(&project.id).execute(&state.pool).await.unwrap();
    assert_eq!(
        capture_with_history(&state, &project.id)
            .await
            .err()
            .unwrap()
            .to_string(),
        "PROJECT_SNAPSHOT_RECOVERY_REQUIRED"
    );
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM project_storage_commits WHERE project_id=?")
            .bind(&project.id)
            .fetch_one(&state.pool)
            .await
            .unwrap();
    assert_eq!(count, 1);
    sqlx::query("DELETE FROM project_storage_commits WHERE project_id=?")
        .bind(&project.id)
        .execute(&state.pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO project_history_events(project_id,kind,created_at) VALUES(?,'edit',CAST(zeroblob(17000000) AS TEXT))")
        .bind(&project.id).execute(&state.pool).await.unwrap();
    assert_eq!(
        capture_with_history(&state, &project.id)
            .await
            .err()
            .unwrap()
            .to_string(),
        "HISTORY_ARCHIVE_TOO_LARGE"
    );
    sqlx::query("DELETE FROM project_history_events WHERE project_id=?")
        .bind(&project.id)
        .execute(&state.pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO project_history(project_id,state_json) VALUES(?,CAST(zeroblob(17000000) AS TEXT))")
        .bind(&project.id).execute(&state.pool).await.unwrap();
    assert_eq!(
        capture_with_history(&state, &project.id)
            .await
            .err()
            .unwrap()
            .to_string(),
        "HISTORY_ARCHIVE_TOO_LARGE"
    );
}
