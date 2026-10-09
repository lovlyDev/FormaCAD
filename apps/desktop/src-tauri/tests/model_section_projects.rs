#![cfg(feature = "native-occt")]
use forma_core::{
    core::AppState,
    models::Project,
    native::{section::SectionPlane, Solid},
};
use std::{
    path::Path,
    sync::{atomic::AtomicBool, Arc},
};
async fn fixture(root: &Path) -> (Arc<AppState>, Project) {
    let (_, guard) = tracing_appender::non_blocking(std::io::sink());
    let state = Arc::new(AppState {
        project_access: Default::default(),
        cad_tasks: Default::default(),
        root: root.into(),
        pool: forma_core::storage::open(&root.join("fixture.sqlite"))
            .await
            .unwrap(),
        grants: tokio::sync::Mutex::new(Default::default()),
        tasks: tokio::sync::Mutex::new(Default::default()),
        writes: tokio::sync::Mutex::new(()),
        _log_guard: guard,
    });
    let source = root.join("fixture.step");
    Solid::box_solid(40., 20., 10.)
        .unwrap()
        .write_step(&source)
        .unwrap();
    let bytes = std::fs::read(source).unwrap();
    let hash = forma_core::artifacts::digest(&bytes);
    let revision = uuid::Uuid::new_v4().to_string();
    let project:Project=serde_json::from_value(serde_json::json!({
        "schemaVersion":1,"id":uuid::Uuid::new_v4().to_string(),"name":"Section fixture","units":"mm","agent":"codex","pinned":false,
        "createdAt":"2026-10-04T00:00:00Z","updatedAt":"2026-10-04T00:00:00Z","currentRevision":revision,"messages":[],"exports":[],
        "files":[{"name":"saved.step","kind":"source","size":bytes.len(),"sha256":hash,"data":forma_core::artifacts::data_url("saved.step",&bytes)}],
        "revisions":[{"id":revision,"parent":null,"createdAt":"2026-10-04T00:00:00Z","prompt":"Saved","source":"saved.step","preview":null,"program":null,"programBase":null,
        "parameters":{"kind":"box","width":40,"depth":20,"height":10,"thickness":2,"holeDiameter":4,"holes":0}}]
    })).unwrap();
    project.validate().unwrap();
    std::fs::create_dir(root.join(&project.id)).unwrap();
    std::fs::write(
        root.join(&project.id).join("sentinel.txt"),
        b"read-only project",
    )
    .unwrap();
    store(&state, &project).await;
    (state, project)
}
async fn store(state: &AppState, project: &Project) {
    sqlx::query("INSERT INTO projects(id,name,payload,updated_at) VALUES(?,?,?,?) ON CONFLICT(id) DO UPDATE SET payload=excluded.payload")
        .bind(&project.id).bind(&project.name).bind(serde_json::to_string(project).unwrap()).bind(&project.updated_at).execute(&state.pool).await.unwrap();
}
fn plane() -> SectionPlane {
    SectionPlane {
        origin_mm: [0., 0., 5.],
        normal: [0., 0., 1.],
        deflection_mm: 0.01,
    }
}

#[tokio::test]
async fn saved_project_section_changes_neither_database_payload_nor_project_folder() {
    let root = tempfile::tempdir().unwrap();
    let (state, project) = fixture(root.path()).await;
    let payload = serde_json::to_string(&project).unwrap();
    let source = std::fs::read(root.path().join("fixture.step")).unwrap();
    let report = forma_core::model_section::build::section_with_executable(
        &state,
        &project.id,
        project.current_revision.as_deref().unwrap(),
        &plane(),
        Arc::new(AtomicBool::new(false)),
        Path::new(env!("CARGO_BIN_EXE_forma-cad-worker")),
    )
    .await
    .unwrap();
    assert!((report.geometry.total_length_mm - 120.).abs() < 1e-6);
    let saved: String = sqlx::query_scalar("SELECT payload FROM projects WHERE id=?")
        .bind(&project.id)
        .fetch_one(&state.pool)
        .await
        .unwrap();
    assert_eq!(saved, payload);
    assert_eq!(
        std::fs::read(root.path().join("fixture.step")).unwrap(),
        source
    );
    assert_eq!(
        std::fs::read_dir(root.path().join(&project.id))
            .unwrap()
            .count(),
        1
    );
    assert_eq!(
        std::fs::read_dir(root.path().join(".transient/sections"))
            .unwrap()
            .count(),
        0
    );
    assert!(state.cad_tasks.status().unwrap().is_empty());
}

#[tokio::test]
async fn stale_head_before_or_after_queue_prevents_worker_and_publication() {
    let root = tempfile::tempdir().unwrap();
    let (state, project) = fixture(root.path()).await;
    let old_revision = project.current_revision.clone().unwrap();
    let worker = Path::new(env!("CARGO_BIN_EXE_forma-cad-worker"));
    let error = forma_core::model_section::build::section_with_executable(
        &state,
        &project.id,
        "stale_revision",
        &plane(),
        Arc::new(AtomicBool::new(false)),
        worker,
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("SECTION_STALE"));
    let blocker = state
        .cad_tasks
        .acquire(
            &project.id,
            "blocking-test",
            Arc::new(AtomicBool::new(false)),
        )
        .await
        .unwrap();
    let task_state = state.clone();
    let task_id = project.id.clone();
    let task_revision = old_revision.clone();
    let task = tokio::spawn(async move {
        forma_core::model_section::build::section_with_executable(
            &task_state,
            &task_id,
            &task_revision,
            &plane(),
            Arc::new(AtomicBool::new(false)),
            Path::new(env!("CARGO_BIN_EXE_forma-cad-worker")),
        )
        .await
    });
    for _ in 0..100 {
        if state
            .cad_tasks
            .status()
            .unwrap()
            .iter()
            .any(|t| t.kind == "section")
        {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    assert!(state
        .cad_tasks
        .status()
        .unwrap()
        .iter()
        .any(|t| t.kind == "section"));
    let mut changed = project.clone();
    let mut revision = changed.revisions[0].clone();
    revision.id = uuid::Uuid::new_v4().to_string();
    revision.parent = Some(old_revision);
    changed.current_revision = Some(revision.id.clone());
    changed.revisions.push(revision);
    store(&state, &changed).await;
    drop(blocker);
    assert!(task
        .await
        .unwrap()
        .unwrap_err()
        .to_string()
        .contains("SECTION_STALE"));
    assert!(!root.path().join(".transient/sections").exists());
    assert_eq!(
        std::fs::read_dir(root.path().join(&project.id))
            .unwrap()
            .count(),
        1
    );
}
