use forma_core::{core::AppState, models::Project};
use serde_json::json;
use std::{collections::BTreeMap, path::Path};

pub async fn state(root: &Path) -> AppState {
    let pool = forma_core::storage::open(&root.join("repository.sqlite"))
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

/// Real persistence/history/CAS route, without a CAD kernel or fabricated SQL history.
pub async fn project(state: &AppState) -> Project {
    let empty: Project = serde_json::from_value(json!({
        "schemaVersion":1,"id":uuid::Uuid::new_v4().to_string(),"name":"Repository fixture",
        "units":"mm","agent":"codex","pinned":false,
        "createdAt":"2026-10-08T00:00:00Z","updatedAt":"2026-10-08T00:00:00Z",
        "revisions":[],"currentRevision":null,"messages":[],"files":[],"exports":[]
    }))
    .unwrap();
    let mut project = forma_core::projects::persist(state, empty, vec![], None)
        .await
        .unwrap();
    for index in 1..=2 {
        let id = uuid::Uuid::new_v4().to_string();
        let name = format!("source-{index}.step");
        // Synthetic sealed bytes: repository tests do not claim STEP geometry validation.
        let bytes = format!("ISO-10303-21; REPOSITORY TEST SOURCE {index}; END-ISO-10303-21;");
        project.files.push(
            serde_json::from_value(json!({
                "name":name,"kind":"source","size":bytes.len(),
                "data":forma_core::artifacts::data_url(&name,bytes.as_bytes())
            }))
            .unwrap(),
        );
        project.revisions.push(
            serde_json::from_value(json!({
                "id":id,"parent":project.current_revision,"createdAt":"2026-10-08T00:01:00Z",
                "prompt":"Committed repository fixture edit",
                "parameters":{"kind":"box","width":80,"depth":60,"height":30,
                    "thickness":3,"holeDiameter":0,"holes":0},"source":name
            }))
            .unwrap(),
        );
        project.current_revision = Some(id);
        let pending = forma_core::artifacts::normalize(&mut project).unwrap();
        project = forma_core::projects::persist(state, project, pending, None)
            .await
            .unwrap();
    }
    project
}

pub fn files(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, path: &Path, result: &mut BTreeMap<String, Vec<u8>>) {
        if !path.exists() {
            return;
        }
        for entry in std::fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, result);
            } else {
                let relative = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                // SELECT may touch SQLite shm state; assert logical DB rows separately.
                if !relative.starts_with("repository.sqlite") {
                    result.insert(relative, std::fs::read(path).unwrap());
                }
            }
        }
    }
    let mut result = BTreeMap::new();
    walk(root, root, &mut result);
    result
}

type Event = (
    i64,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
);
type Journal = (Option<String>, String, String);
#[derive(Debug, PartialEq)]
pub struct LogicalSnapshot {
    pub payload: String,
    pub history: String,
    pub events: Vec<Event>,
    pub journal: Option<Journal>,
    pub table_counts: Vec<i64>,
    pub files: BTreeMap<String, Vec<u8>>,
}

pub async fn snapshot(state: &AppState, id: &str) -> LogicalSnapshot {
    let payload = sqlx::query_scalar("SELECT payload FROM projects WHERE id=?")
        .bind(id)
        .fetch_one(&state.pool)
        .await
        .unwrap();
    let history = sqlx::query_scalar("SELECT state_json FROM project_history WHERE project_id=?")
        .bind(id)
        .fetch_one(&state.pool)
        .await
        .unwrap();
    let events = sqlx::query_as("SELECT id,kind,from_revision,to_revision,target_revision,created_at FROM project_history_events WHERE project_id=? ORDER BY id")
        .bind(id).fetch_all(&state.pool).await.unwrap();
    let journal = sqlx::query_as("SELECT base_manifest_sha256,target_manifest,project_sha256 FROM project_storage_commits WHERE project_id=?")
        .bind(id).fetch_optional(&state.pool).await.unwrap();
    let mut table_counts = Vec::new();
    for table in [
        "projects",
        "project_history",
        "project_history_events",
        "project_storage_commits",
        "settings",
        "permissions",
        "agent_sessions",
        "activities",
    ] {
        table_counts.push(
            sqlx::query_scalar::<_, i64>(&format!("SELECT COUNT(*) FROM {table}"))
                .fetch_one(&state.pool)
                .await
                .unwrap(),
        );
    }
    LogicalSnapshot {
        payload,
        history,
        events,
        journal,
        table_counts,
        files: files(&state.root),
    }
}

pub async fn history(state: &AppState, project: &Project) -> Vec<u8> {
    let mut transaction = state.pool.begin().await.unwrap();
    let bytes = forma_core::project_history::export_history(&mut transaction, project)
        .await
        .unwrap();
    transaction.rollback().await.unwrap();
    bytes
}

pub async fn pending(state: &AppState, project: &Project) {
    sqlx::query("INSERT INTO project_storage_commits(project_id,base_manifest_sha256,target_manifest,project_sha256) VALUES(?,?,?,?)")
        .bind(&project.id).bind("a".repeat(64)).bind("{}")
        .bind(forma_core::artifacts::digest(&serde_json::to_vec(project).unwrap()))
        .execute(&state.pool).await.unwrap();
}

/// SQL-only legacy fixture: inline data is retained, no folder/blob is materialized.
pub async fn legacy_inline(state: &AppState, wrong_size: bool) -> Project {
    let id = uuid::Uuid::new_v4().to_string();
    let revision = uuid::Uuid::new_v4().to_string();
    let bytes = b"SYNTHETIC LEGACY INLINE STEP BYTES";
    let project: Project = serde_json::from_value(json!({
        "schemaVersion":1,"id":id,"name":"Inline legacy fixture","units":"mm",
        "agent":"codex","pinned":false,"createdAt":"2026-10-08T00:00:00Z",
        "updatedAt":"2026-10-08T00:00:00Z","currentRevision":revision,
        "messages":[],"exports":[],
        "files":[{"name":"legacy.step","kind":"source",
            "size":if wrong_size {bytes.len()+1} else {bytes.len()},
            "data":forma_core::artifacts::data_url("legacy.step",bytes),
            "sha256":if wrong_size {forma_core::artifacts::digest(bytes)} else {"0".repeat(64)}}],
        "revisions":[{"id":revision,"parent":null,"createdAt":"2026-10-08T00:00:00Z",
            "prompt":"Legacy committed fixture","source":"legacy.step",
            "parameters":{"kind":"box","width":80,"depth":60,"height":30,
                "thickness":3,"holeDiameter":0,"holes":0}}]
    }))
    .unwrap();
    project.validate().unwrap();
    let mut transaction = state.pool.begin().await.unwrap();
    sqlx::query("INSERT INTO projects(id,name,payload,updated_at) VALUES(?,?,?,?)")
        .bind(&project.id)
        .bind(&project.name)
        .bind(serde_json::to_string(&project).unwrap())
        .bind(&project.updated_at)
        .execute(&mut *transaction)
        .await
        .unwrap();
    forma_core::project_history::record_commit(&mut transaction, None, &project, None)
        .await
        .unwrap();
    transaction.commit().await.unwrap();
    project
}
