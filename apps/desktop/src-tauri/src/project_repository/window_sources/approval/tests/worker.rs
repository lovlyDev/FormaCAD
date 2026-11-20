//! Explicit genuine-worker proof of request -> decision -> original execution.
use super::*;
use crate::cad_tasks::CadTaskGate;
use std::{path::PathBuf, process::Command, sync::atomic::AtomicBool};

fn program(width: f64) -> String {
    serde_json::json!({"schemaVersion":2,"revisionId":"approval_candidate","parameters":[],"features":[
        {"id":"profile","name":"Profile","operation":{"type":"rectangle","width":{"kind":"literal","mm":width},"depth":{"kind":"literal","mm":20}}},
        {"id":"pad","name":"Pad","operation":{"type":"extrude","sketchId":"profile","distance":{"kind":"literal","mm":10}}}
    ],"bodies":[{"id":"body","name":"Body","sourceFeatureId":"pad"}]}).to_string()
}

async fn apply(
    windows: &WindowSources,
    source: ApprovalSource<'_>,
    intent: WriteIntent,
    runtime: ModelRuntime<'_>,
) -> String {
    let command = intent.command();
    let id = windows.request_write(source, intent).await.unwrap();
    windows
        .resolve_write(&id, source.window, true)
        .await
        .unwrap();
    let reply = windows
        .execute_write(&id, source, command, runtime)
        .await
        .unwrap();
    assert!(windows.consume_write(&id, source).await.is_err());
    let reply = serde_json::to_value(reply).unwrap();
    assert_eq!(reply["status"], "committed");
    assert_eq!(reply["sourceId"], source.source_id);
    assert_eq!(reply["projectId"], source.project_id);
    assert_eq!(reply["baseCommitId"], source.commit_id);
    let saved = reply["commitId"].as_str().unwrap();
    assert_ne!(saved, source.commit_id);
    saved.to_owned()
}

async fn inspect(
    windows: &WindowSources,
    source: ApprovalSource<'_>,
    width: f64,
    executable: &std::path::Path,
) {
    let read = windows
        .capture_read(
            source.window,
            source.source_id,
            source.project_id,
            source.commit_id,
        )
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
        &f.windows,
        f.source(),
        WriteIntent::Model(ModelIntent {
            program: program(40.),
            prompt: "First retained command".into(),
        }),
        runtime(),
    )
    .await;
    let first_source = ApprovalSource {
        commit_id: &first,
        ..f.source()
    };
    inspect(&f.windows, first_source, 40., &executable).await;
    let second = apply(
        &f.windows,
        first_source,
        WriteIntent::Model(ModelIntent {
            program: program(80.),
            prompt: "Second retained command".into(),
        }),
        runtime(),
    )
    .await;
    let before_close = ApprovalSource {
        commit_id: &second,
        ..f.source()
    };
    inspect(&f.windows, before_close, 80., &executable).await;
    assert!(tasks.lock().await.is_empty());
    assert!(gate.status().unwrap().is_empty());
    assert!(f
        .windows
        .close_source(before_close.window, before_close.source_id)
        .unwrap());
    assert!(f
        .windows
        .capture_read(
            before_close.window,
            before_close.source_id,
            before_close.project_id,
            before_close.commit_id
        )
        .await
        .is_err());

    let nonce = uuid::Uuid::new_v4().to_string();
    let child_test = format!(
        "{}::reopen_history_child",
        module_path!().split_once("::").unwrap().1
    );
    let child = Command::new(std::env::current_exe().unwrap())
        .args(["--ignored", "--exact", &child_test, "--nocapture"])
        .env("FORMA_FOLDER_TEST_WORKER", &executable)
        .env("FORMA_REOPEN_HISTORY_PATH", f.root.path())
        .env("FORMA_REOPEN_HISTORY_PROJECT", &f.id)
        .env("FORMA_REOPEN_HISTORY_COMMIT", &second)
        .env("FORMA_REOPEN_HISTORY_CLOSED_SOURCE", &f.key)
        .env(
            "FORMA_REOPEN_HISTORY_PARENT_PID",
            std::process::id().to_string(),
        )
        .env("FORMA_REOPEN_HISTORY_NONCE", &nonce)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&child.stdout);
    assert!(
        child.status.success(),
        "{stdout}\n{}",
        String::from_utf8_lossy(&child.stderr)
    );
    let marker = format!("FOLDER_CAD_REOPEN_HISTORY_OK:{nonce}");
    assert!(stdout.lines().any(|line| line == marker), "{stdout}");
    println!("\nFOLDER_CAD_REOPEN_HISTORY_OK");
}

#[tokio::test]
#[ignore = "exact fresh-process probe for the real CAD history test"]
async fn reopen_history_child() {
    let root = PathBuf::from(std::env::var_os("FORMA_REOPEN_HISTORY_PATH").unwrap());
    let executable = PathBuf::from(std::env::var_os("FORMA_FOLDER_TEST_WORKER").unwrap());
    let id = std::env::var("FORMA_REOPEN_HISTORY_PROJECT").unwrap();
    let expected_commit = std::env::var("FORMA_REOPEN_HISTORY_COMMIT").unwrap();
    let closed_key = std::env::var("FORMA_REOPEN_HISTORY_CLOSED_SOURCE").unwrap();
    let parent_pid: u32 = std::env::var("FORMA_REOPEN_HISTORY_PARENT_PID")
        .unwrap()
        .parse()
        .unwrap();
    assert_ne!(std::process::id(), parent_pid);

    let windows = WindowSources::default();
    let opened = windows
        .for_window("window")
        .unwrap()
        .open(&root)
        .await
        .unwrap();
    let key = opened.id.to_string();
    assert_ne!(key, closed_key);
    assert_eq!(opened.snapshot.project().id, id);
    assert_eq!(opened.snapshot.commit_id(), expected_commit);
    assert!(windows
        .capture_read("window", &closed_key, &id, &expected_commit)
        .await
        .is_err());
    let reopened = ApprovalSource {
        window: "window",
        source_id: &key,
        project_id: &id,
        commit_id: &expected_commit,
    };
    inspect(&windows, reopened, 80., &executable).await;

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
    let undo = apply(
        &windows,
        reopened,
        WriteIntent::History(Direction::Undo),
        runtime(),
    )
    .await;
    let undone = ApprovalSource {
        commit_id: &undo,
        ..reopened
    };
    inspect(&windows, undone, 40., &executable).await;
    let redo = apply(
        &windows,
        undone,
        WriteIntent::History(Direction::Redo),
        runtime(),
    )
    .await;
    inspect(
        &windows,
        ApprovalSource {
            commit_id: &redo,
            ..reopened
        },
        80.,
        &executable,
    )
    .await;
    assert!(tasks.lock().await.is_empty());
    assert!(gate.status().unwrap().is_empty());
    println!(
        "\nFOLDER_CAD_REOPEN_HISTORY_OK:{}",
        std::env::var("FORMA_REOPEN_HISTORY_NONCE").unwrap()
    );
}
