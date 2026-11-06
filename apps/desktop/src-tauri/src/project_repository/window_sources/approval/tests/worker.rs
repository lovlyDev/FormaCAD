//! Explicit genuine-worker proof of request -> decision -> original execution.
use super::*;
use crate::{cad_tasks::CadTaskGate, project_repository::sqlite_folder::api::SavedCommit};
use std::sync::atomic::AtomicBool;

fn program(width: f64) -> String {
    serde_json::json!({"schemaVersion":2,"revisionId":"approval_candidate","parameters":[],"features":[
        {"id":"profile","name":"Profile","operation":{"type":"rectangle","width":{"kind":"literal","mm":width},"depth":{"kind":"literal","mm":20}}},
        {"id":"pad","name":"Pad","operation":{"type":"extrude","sketchId":"profile","distance":{"kind":"literal","mm":10}}}
    ],"bodies":[{"id":"body","name":"Body","sourceFeatureId":"pad"}]}).to_string()
}

async fn apply(
    f: &Fixture,
    commit: &str,
    intent: WriteIntent,
    runtime: ModelRuntime<'_>,
) -> SavedCommit {
    let source = ApprovalSource {
        commit_id: commit,
        ..f.source()
    };
    let id = f.windows.request_write(source, intent).await.unwrap();
    f.windows.resolve_write(&id, "window", true).await.unwrap();
    let approved = f.windows.consume_write(&id, source).await.unwrap();
    assert!(f.windows.consume_write(&id, source).await.is_err());
    match approved.execute(runtime).await.unwrap() {
        ModelChange::Published(Publication::Committed(saved)) => saved,
        _ => panic!("expected a confirmed original-command publication"),
    }
}

async fn inspect(f: &Fixture, saved: &SavedCommit, width: f64, executable: &std::path::Path) {
    let read = f
        .windows
        .capture_read("window", &f.key, &f.id, saved.commit_id())
        .await
        .unwrap();
    let revision = read
        .project()
        .revisions
        .iter()
        .find(|revision| Some(&revision.id) == read.project().current_revision.as_ref())
        .unwrap();
    let file = read
        .project()
        .files
        .iter()
        .find(|file| Some(&file.name) == revision.source.as_ref())
        .unwrap();
    let bytes = read.read_asset(file).await.unwrap();
    let stage = tempfile::tempdir().unwrap();
    std::fs::write(stage.path().join("source.step"), bytes).unwrap();
    let report = crate::native::inspection::inspect_with_executable(
        stage.path(),
        Arc::new(AtomicBool::new(false)),
        executable,
    )
    .await
    .unwrap();
    assert!((report.volume_mm3 - width * 20. * 10.).abs() < 1e-6);
    for (actual, expected) in report.bounds_mm.into_iter().zip([width, 20., 10.]) {
        assert!((actual - expected).abs() < 1e-6);
    }
    read.recheck().await.unwrap();
}

#[tokio::test]
#[ignore = "requires explicitly built real CAD worker"]
async fn real_command_decision_applies_original_cad_and_history() {
    let executable: std::path::PathBuf = std::env::var_os("FORMA_FOLDER_TEST_WORKER")
        .expect("explicit genuine worker")
        .into();
    let f = Fixture::new().await;
    let work = tempfile::tempdir().unwrap();
    let tasks = Mutex::new(HashMap::new());
    let gate = CadTaskGate::default();
    let runtime = || ModelRuntime {
        tasks: &tasks,
        gate: &gate,
        runtime_root: work.path(),
        transient_root: work.path(),
        executable: Some(&executable),
    };
    let first = apply(
        &f,
        &f.commit,
        WriteIntent::Model(ModelIntent {
            program: program(40.),
            prompt: "First retained command".into(),
        }),
        runtime(),
    )
    .await;
    inspect(&f, &first, 40., &executable).await;
    let second = apply(
        &f,
        first.commit_id(),
        WriteIntent::Model(ModelIntent {
            program: program(80.),
            prompt: "Second retained command".into(),
        }),
        runtime(),
    )
    .await;
    inspect(&f, &second, 80., &executable).await;
    let undo = apply(
        &f,
        second.commit_id(),
        WriteIntent::History(Direction::Undo),
        runtime(),
    )
    .await;
    inspect(&f, &undo, 40., &executable).await;
    let redo = apply(
        &f,
        undo.commit_id(),
        WriteIntent::History(Direction::Redo),
        runtime(),
    )
    .await;
    inspect(&f, &redo, 80., &executable).await;
    assert!(tasks.lock().await.is_empty());
    assert!(gate.status().unwrap().is_empty());
}
