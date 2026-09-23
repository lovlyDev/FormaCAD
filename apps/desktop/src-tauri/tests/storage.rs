use forma_core::{
    models::{Parameters, Project, Revision},
    projects::validate_history,
    storage,
};
fn project() -> Project {
    let r = Revision {
        program: None,
        program_base: None,
        id: uuid::Uuid::new_v4().to_string(),
        parent: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        prompt: "Created bracket".into(),
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
    };
    Project {
        schema_version: 1,
        id: uuid::Uuid::new_v4().to_string(),
        name: "Bracket".into(),
        units: "mm".into(),
        agent: "codex".into(),
        pinned: false,
        thumbnail: None,
        thumbnail_revision: None,
        created_at: r.created_at.clone(),
        updated_at: r.created_at.clone(),
        current_revision: Some(r.id.clone()),
        revisions: vec![r],
        messages: vec![],
        files: vec![],
        exports: vec![],
    }
}
#[tokio::test]
async fn migrations_and_crash_recovery() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("db.sqlite");
    let pool = storage::open(&path).await.unwrap();
    sqlx::query("INSERT INTO agent_sessions(id,project_id,status,created_at) VALUES('s','p','running','now')").execute(&pool).await.unwrap();
    pool.close().await;
    let pool = storage::open(&path).await.unwrap();
    let status: String = sqlx::query_scalar("SELECT status FROM agent_sessions WHERE id='s'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "interrupted");
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM activities WHERE kind='recovery'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
    pool.close().await;
}
#[tokio::test]
async fn corrupt_database_is_not_replaced() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("bad.sqlite");
    std::fs::write(&path, b"not a database").unwrap();
    assert!(storage::open(&path).await.is_err());
    assert_eq!(std::fs::read(path).unwrap(), b"not a database");
}
#[test]
fn immutable_history_and_restore() {
    let old = project();
    let mut next = old.clone();
    next.revisions[0].parameters.width = 300.0;
    assert!(validate_history(&old, &next).is_err());
    next = old.clone();
    let mut restored = old.revisions[0].clone();
    restored.id = uuid::Uuid::new_v4().to_string();
    restored.parent = old.current_revision.clone();
    next.current_revision = Some(restored.id.clone());
    restored.parameters.width = 140.0;
    next.revisions.push(restored);
    assert!(validate_history(&old, &next).unwrap());
    next.validate().unwrap();
}
#[test]
fn missing_parent_rejected() {
    let mut p = project();
    p.revisions[0].parent = Some(uuid::Uuid::new_v4().to_string());
    assert!(p.validate().is_err());
}

#[test]
fn repeated_geometry_is_not_a_revision() {
    let old = project();
    let mut next = old.clone();
    let mut r = old.revisions[0].clone();
    r.id = uuid::Uuid::new_v4().to_string();
    r.parent = old.current_revision.clone();
    next.current_revision = Some(r.id.clone());
    next.revisions.push(r);
    assert!(validate_history(&old, &next).is_err());
}
