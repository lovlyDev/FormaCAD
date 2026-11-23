//! This test is compiled inside a temporary checkout of the exact 1.1.0 release.
use forma_core::{artifacts, core::AppState, models::Project, projects, storage};
use std::path::PathBuf;

#[tokio::test]
#[ignore = "invoked by the FCP1 native STEP transfer regression"]
async fn write_legacy_1_1_project() {
    assert_eq!(env!("CARGO_PKG_VERSION"), "1.1.0");
    let data = PathBuf::from(std::env::var_os("FORMA_LEGACY_1_1_DATA").unwrap());
    let input = PathBuf::from(std::env::var_os("FORMA_LEGACY_1_1_STEP").unwrap());
    let project_id = std::env::var("FORMA_LEGACY_1_1_ID").unwrap();
    assert!(data.is_absolute());
    assert_eq!(std::fs::read_dir(&data).unwrap().count(), 0);
    let root = data.join("projects");
    std::fs::create_dir(&root).unwrap();
    let pool = storage::open(&data.join("forma.sqlite")).await.unwrap();
    let (_, guard) = tracing_appender::non_blocking(std::io::sink());
    let state = AppState {
        root,
        pool,
        grants: Default::default(),
        tasks: Default::default(),
        writes: Default::default(),
        _log_guard: guard,
    };
    let bytes = std::fs::read(input).unwrap();
    let revision_id = uuid::Uuid::new_v4().to_string();
    let revision = revision_id.as_str();
    let now = "2026-01-01T00:00:00Z";
    let mut project: Project = serde_json::from_value(serde_json::json!({
        "schemaVersion": 1, "id": project_id, "name": "Legacy 1.1 STEP fixture",
        "units": "mm", "agent": "codex", "pinned": false, "createdAt": now, "updatedAt": now,
        "currentRevision": revision, "exports": [], "messages": [],
        "files": [{"name": "selected.step", "kind": "model", "size": bytes.len(),
            "data": artifacts::data_url("selected.step", &bytes)}],
        "revisions": [{"id": revision, "parent": null, "createdAt": now, "prompt": "Legacy model",
            "source": "selected.step", "parameters": {
                "kind": "box", "width": 40, "depth": 20, "height": 10,
                "thickness": 1, "holeDiameter": 0, "holes": 0
            }}]
    }))
    .unwrap();
    let pending = artifacts::normalize(&mut project).unwrap();
    let saved = projects::persist(&state, project, pending, None)
        .await
        .unwrap();
    let reloaded = projects::get(&state, &saved.id).await.unwrap();
    assert_eq!(reloaded.current_revision.as_deref(), Some(revision));
    assert_eq!(
        artifacts::read(&state, &saved.id, &reloaded.files[0]).unwrap(),
        bytes
    );
    state.pool.close().await;
    println!("\nFORMA_LEGACY_1_1_FIXTURE_OK");
}
