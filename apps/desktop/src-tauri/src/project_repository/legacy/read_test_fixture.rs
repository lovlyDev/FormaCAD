//! Real isolated SQL fixtures for read-only consumers; no native CAD claims.
use crate::{core::AppState, models::Project};
use std::{collections::BTreeMap, path::Path};

pub async fn state(root: &Path) -> AppState {
    let pool = crate::storage::open(&root.join("read.sqlite"))
        .await
        .unwrap();
    let (_, guard) = tracing_appender::non_blocking(std::io::sink());
    AppState {
        root: root.into(),
        pool,
        _log_guard: guard,
        cad_tasks: Default::default(),
        project_access: Default::default(),
        grants: Default::default(),
        tasks: Default::default(),
        writes: Default::default(),
    }
}

pub async fn project(state: &AppState, size: usize) -> Project {
    let revision = uuid::Uuid::new_v4().to_string();
    let bytes = vec![b'x'; size];
    let project: Project = serde_json::from_value(serde_json::json!({
        "schemaVersion":1,"id":uuid::Uuid::new_v4().to_string(),"name":"Read fixture",
        "units":"mm","agent":"codex","pinned":false,
        "createdAt":"2026-10-09T00:00:00Z","updatedAt":"2026-10-09T00:00:00Z",
        "currentRevision":revision,"messages":[],"exports":[],
        "files":[{"name":"model.step","kind":"source","size":size,
            "data":crate::artifacts::data_url("model.step",&bytes),
            "sha256":crate::artifacts::digest(&bytes)}],
        "revisions":[{"id":revision,"parent":null,"createdAt":"2026-10-09T00:00:00Z",
            "prompt":"Synthetic saved model","source":"model.step",
            "parameters":{"kind":"box","width":40,"depth":20,"height":10,
                "thickness":2,"holeDiameter":0,"holes":0}}]
    }))
    .unwrap();
    replace(state, &project, true).await;
    project
}

pub async fn replace(state: &AppState, project: &Project, initial: bool) {
    project.validate().unwrap();
    let mut transaction = state.pool.begin().await.unwrap();
    sqlx::query("INSERT INTO projects(id,name,payload,updated_at) VALUES(?,?,?,?) ON CONFLICT(id) DO UPDATE SET name=excluded.name,payload=excluded.payload,updated_at=excluded.updated_at")
        .bind(&project.id).bind(&project.name).bind(serde_json::to_string(project).unwrap())
        .bind(&project.updated_at).execute(&mut *transaction).await.unwrap();
    if initial {
        crate::project_history::record_commit(&mut transaction, None, project, None)
            .await
            .unwrap();
    }
    transaction.commit().await.unwrap();
}

pub async fn pending(state: &AppState, id: &str) {
    sqlx::query("INSERT INTO project_storage_commits(project_id,base_manifest_sha256,target_manifest,project_sha256) VALUES(?,NULL,?,?)")
        .bind(id).bind("NOT A RECOVERABLE MANIFEST").bind("a".repeat(64))
        .execute(&state.pool).await.unwrap();
}

pub async fn next_head(state: &AppState, old: &Project) -> Project {
    let mut next = old.clone();
    let mut revision = old.revisions.last().unwrap().clone();
    revision.id = uuid::Uuid::new_v4().to_string();
    revision.parent = old.current_revision.clone();
    next.current_revision = Some(revision.id.clone());
    next.revisions.push(revision);
    let mut transaction = state.pool.begin().await.unwrap();
    sqlx::query("UPDATE projects SET payload=? WHERE id=?")
        .bind(serde_json::to_string(&next).unwrap())
        .bind(&next.id)
        .execute(&mut *transaction)
        .await
        .unwrap();
    crate::project_history::record_commit(&mut transaction, Some(old), &next, None)
        .await
        .unwrap();
    transaction.commit().await.unwrap();
    next
}

pub async fn logical(state: &AppState, id: &str) -> (String, String, i64, i64) {
    let project: String = sqlx::query_scalar("SELECT payload FROM projects WHERE id=?")
        .bind(id)
        .fetch_one(&state.pool)
        .await
        .unwrap();
    let history: String =
        sqlx::query_scalar("SELECT state_json FROM project_history WHERE project_id=?")
            .bind(id)
            .fetch_one(&state.pool)
            .await
            .unwrap();
    let journals: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM project_storage_commits")
        .fetch_one(&state.pool)
        .await
        .unwrap();
    let events: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM project_history_events")
        .fetch_one(&state.pool)
        .await
        .unwrap();
    (project, history, journals, events)
}

pub fn files(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, current: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in std::fs::read_dir(current).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                let relative = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                if !relative.starts_with("read.sqlite") {
                    out.insert(relative, std::fs::read(path).unwrap());
                }
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}
