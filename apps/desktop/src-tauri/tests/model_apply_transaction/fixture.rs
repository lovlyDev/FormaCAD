use forma_core::{
    core::AppState,
    modeling::apply::{apply_with_executable, ApplyRequest},
    models::Project,
};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

pub fn executable() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_forma-cad-worker"))
}

pub fn document(width: f64) -> Value {
    json!({"schemaVersion":2,"revisionId":"candidate","parameters":[],"features":[
        {"id":"profile","name":"Profile","operation":{"type":"rectangle","width":{"kind":"literal","mm":width},"depth":{"kind":"literal","mm":20}}},
        {"id":"pad","name":"Pad","operation":{"type":"extrude","sketchId":"profile","distance":{"kind":"literal","mm":10}}}
    ],"bodies":[{"id":"body","name":"Body","sourceFeatureId":"pad"}]})
}
pub fn fillet(owner: &str, path: Value, radius: f64) -> Value {
    let mut value = document(40.);
    value["features"].as_array_mut().unwrap().extend([
        json!({"id":"other_pad","name":"Another source","operation":{"type":"extrude","sketchId":"profile","distance":{"kind":"literal","mm":10}}}),
        json!({"id":"turned","name":"Turn","operation":{"type":"rotate","bodyFeatureId":"pad","axisOriginMm":[0,0,0],"axisDirection":[0,0,1],"angleDeg":17}}),
        json!({"id":"round","name":"Round","operation":{"type":"filletReferencedEdge","bodyFeatureId":"turned",
            "reference":{"schemaVersion":1,"kind":"edge","ownerFeatureId":owner,"role":"box-edge:x:ymin:zmax","occurrencePath":path},
            "radius":{"kind":"literal","mm":radius}}}),
    ]);
    value["bodies"][0]["sourceFeatureId"] = "round".into();
    value
}
pub fn request(project: &Project, value: Value) -> ApplyRequest {
    ApplyRequest::new(
        project.id.clone(),
        value.to_string(),
        "Host transaction test".into(),
        project.current_revision.clone(),
    )
}

pub async fn state(root: &Path) -> Arc<AppState> {
    let pool = forma_core::storage::open(&root.join("apply.sqlite"))
        .await
        .unwrap();
    let (_, guard) = tracing_appender::non_blocking(std::io::sink());
    Arc::new(AppState {
        root: root.into(),
        pool,
        _log_guard: guard,
        cad_tasks: Default::default(),
        project_access: Default::default(),
        grants: Default::default(),
        tasks: Default::default(),
        writes: Default::default(),
    })
}
pub async fn automatic(state: &AppState) {
    sqlx::query("INSERT INTO settings(key,value) VALUES('confirmations',?) ON CONFLICT(key) DO UPDATE SET value=excluded.value")
        .bind(r#"{"mode":"none","overrides":{}}"#).execute(&state.pool).await.unwrap();
}
pub async fn fixture(root: &Path) -> (Arc<AppState>, Project) {
    let state = state(root).await;
    automatic(&state).await;
    let project: Project = serde_json::from_value(json!({"schemaVersion":1,"id":uuid::Uuid::new_v4().to_string(),
        "name":"Apply fixture","units":"mm","agent":"codex","pinned":false,"createdAt":"2026-10-05T00:00:00Z","updatedAt":"2026-10-05T00:00:00Z",
        "currentRevision":null,"revisions":[],"files":[],"messages":[],"exports":[]})).unwrap();
    let empty = forma_core::projects::persist(&state, project, vec![], None)
        .await
        .unwrap();
    let first = apply_with_executable(&state, request(&empty, document(40.)), executable())
        .await
        .unwrap();
    assert!(first.committed);
    (state, first.project)
}

type HistoryEventRow = (
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
);

#[derive(Debug, PartialEq)]
pub struct Snapshot {
    project: String,
    history: String,
    events: Vec<HistoryEventRow>,
    journal: i64,
    files: BTreeMap<String, String>,
}
fn directory(root: &Path, path: &Path, result: &mut BTreeMap<String, String>) {
    if !path.exists() {
        return;
    }
    for entry in std::fs::read_dir(path).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            directory(root, &path, result);
        } else {
            result.insert(
                path.strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
                forma_core::artifacts::digest(&std::fs::read(path).unwrap()),
            );
        }
    }
}
pub async fn snapshot(state: &AppState, project_id: &str) -> Snapshot {
    let project = sqlx::query_scalar("SELECT payload FROM projects WHERE id=?")
        .bind(project_id)
        .fetch_one(&state.pool)
        .await
        .unwrap();
    let history = sqlx::query_scalar("SELECT state_json FROM project_history WHERE project_id=?")
        .bind(project_id)
        .fetch_one(&state.pool)
        .await
        .unwrap();
    let events = sqlx::query_as("SELECT kind,from_revision,to_revision,target_revision,created_at FROM project_history_events WHERE project_id=? ORDER BY id")
        .bind(project_id).fetch_all(&state.pool).await.unwrap();
    let journal =
        sqlx::query_scalar("SELECT COUNT(*) FROM project_storage_commits WHERE project_id=?")
            .bind(project_id)
            .fetch_one(&state.pool)
            .await
            .unwrap();
    let mut files = BTreeMap::new();
    directory(&state.root, &state.root.join(project_id), &mut files);
    directory(&state.root, &state.root.join(".blobs"), &mut files);
    Snapshot {
        project,
        history,
        events,
        journal,
        files,
    }
}
pub async fn clean(state: &AppState) {
    assert!(state.tasks.lock().await.is_empty());
    assert!(state.cad_tasks.status().unwrap().is_empty());
    let transient = state.root.join(".transient/model-builds");
    if transient.exists() {
        assert_eq!(std::fs::read_dir(transient).unwrap().count(), 0);
    }
}
pub async fn queued(state: &AppState) {
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while !state
            .cad_tasks
            .status()
            .unwrap()
            .iter()
            .any(|task| task.kind == "model_build" && task.state == "queued")
        {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}
pub async fn cancel(state: &AppState, project_id: &str) {
    state.cad_tasks.cancel_project(project_id).unwrap();
    state
        .tasks
        .lock()
        .await
        .get(project_id)
        .unwrap()
        .store(true, Ordering::Relaxed);
}
pub fn token() -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(false))
}
