use crate::{artifacts, models::Project, project_history};
use sqlx::SqlitePool;
use std::{collections::HashMap, path::Path};

pub(super) async fn persist(pool: &SqlitePool, old: Option<&Project>, new: &Project) {
    new.validate().unwrap();
    let mut transaction = pool.begin().await.unwrap();
    sqlx::query("INSERT INTO projects(id,name,payload,updated_at) VALUES(?,?,?,?) ON CONFLICT(id) DO UPDATE SET name=excluded.name,payload=excluded.payload,updated_at=excluded.updated_at")
        .bind(&new.id).bind(&new.name).bind(serde_json::to_string(new).unwrap()).bind(&new.updated_at)
        .execute(&mut *transaction).await.unwrap();
    project_history::record_commit(&mut transaction, old, new, None)
        .await
        .unwrap();
    transaction.commit().await.unwrap();
}

pub(super) async fn read_project(pool: &SqlitePool, id: &str) -> Project {
    let raw: String = sqlx::query_scalar("SELECT payload FROM projects WHERE id=?")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap();
    let project: Project = serde_json::from_str(&raw).unwrap();
    project.validate().unwrap();
    project
}

pub(super) async fn build(
    root: &Path,
    revisions: usize,
    attachments: usize,
    asset_bytes: usize,
) -> (SqlitePool, Project, HashMap<String, Vec<u8>>) {
    let mut files = Vec::new();
    let mut assets = HashMap::new();
    for index in 0..attachments {
        // Each pair refers to identical bytes under different safe filenames.
        let mut bytes = vec![b'x'; asset_bytes];
        bytes[..8].copy_from_slice(&((index / 2) as u64).to_le_bytes());
        let hash = artifacts::digest(&bytes);
        files.push(serde_json::json!({"name":format!("synthetic-{index}.step"),"kind":"source","size":asset_bytes,
            "data":artifacts::data_url("synthetic.step",&bytes),"sha256":hash}));
        assets.insert(hash, bytes);
    }
    let messages:Vec<_>=(0..revisions).map(|index|serde_json::json!({"id":uuid::Uuid::new_v4().to_string(),
        "role":"user","text":format!("Synthetic storage fixture message {index}: {}","chat ".repeat(64)),
        "createdAt":"2026-10-29T00:00:00Z"})).collect();
    let mut project: Project = serde_json::from_value(
        serde_json::json!({"schemaVersion":1,"id":uuid::Uuid::new_v4().to_string(),
        "name":"Storage benchmark fixture","units":"mm","agent":"codex","pinned":false,
        "createdAt":"2026-10-29T00:00:00Z","updatedAt":"2026-10-29T00:00:00Z",
        "currentRevision":null,"revisions":[],"messages":messages,"files":files,"exports":[]}),
    )
    .unwrap();
    project.validate().unwrap();
    let normalized = artifacts::normalize(&mut project).unwrap();
    assert_eq!(normalized.len(), attachments);
    project.validate().unwrap();
    for (hash, bytes) in &assets {
        artifacts::immutable_write(root, Path::new(&format!("assets/{hash}")), bytes).unwrap();
    }
    let pool = super::candidate::create(&root.join("project.sqlite"))
        .await
        .unwrap();
    persist(&pool, None, &project).await;
    for index in 0..revisions {
        let old = project.clone();
        let revision=serde_json::from_value(serde_json::json!({"id":uuid::Uuid::new_v4().to_string(),
            "parent":project.current_revision,"createdAt":"2026-10-29T00:00:00Z",
            "prompt":format!("Synthetic saved revision {index}"),"source":"synthetic-0.step",
            "parameters":{"kind":"box","width":40+index,"depth":20,"height":10,"thickness":2,"holeDiameter":0,"holes":0}})).unwrap();
        project.revisions.push(revision);
        project.current_revision = Some(project.revisions.last().unwrap().id.clone());
        persist(&pool, Some(&old), &project).await;
    }
    (pool, project, assets)
}

pub(super) fn reject_limits() {
    let mut invalid:Project=serde_json::from_value(serde_json::json!({"schemaVersion":1,"id":uuid::Uuid::new_v4().to_string(),
        "name":"Limits","units":"mm","agent":"codex","pinned":false,"createdAt":"2026-10-29T00:00:00Z",
        "updatedAt":"2026-10-29T00:00:00Z","currentRevision":null,"revisions":[],"messages":[],
        "files":[{"name":"oversize.step","kind":"source","size":artifacts::MAX_FILE_BYTES+1,"sha256":"a".repeat(64)}],"exports":[]})).unwrap();
    assert!(invalid.validate().is_err());
    println!(
        "STORAGE_BENCHMARK {}",
        serde_json::json!({"kind":"rejection","operation":"oversize_declared_asset",
        "declared_bytes":artifacts::MAX_FILE_BYTES+1,"accepted":false,"allocation":"metadata-only; no oversize buffer"})
    );
    invalid.files[0].size = 1;
    invalid.files[0].name = "../escape.step".into();
    assert!(invalid.validate().is_err());
    println!(
        "STORAGE_BENCHMARK {}",
        serde_json::json!({"kind":"rejection","operation":"traversal_metadata","accepted":false})
    );
}
