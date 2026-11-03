//! Explicit real-worker proof of the public host API, including fresh-process read.
use super::*;
use crate::cad_tasks::CadTaskGate;
use std::{
    collections::HashMap,
    path::Path,
    sync::{atomic::AtomicBool, Arc},
};
use tokio::sync::Mutex;

fn program(width: f64) -> String {
    serde_json::json!({"schemaVersion":2,"revisionId":"host_candidate","parameters":[],"features":[
        {"id":"profile","name":"Profile","operation":{"type":"rectangle","width":{"kind":"literal","mm":width},"depth":{"kind":"literal","mm":20}}},
        {"id":"pad","name":"Pad","operation":{"type":"extrude","sketchId":"profile","distance":{"kind":"literal","mm":10}}}
    ],"bodies":[{"id":"body","name":"Body","sourceFeatureId":"pad"}]}).to_string()
}
fn worker() -> std::path::PathBuf {
    std::env::var_os("FORMA_FOLDER_TEST_WORKER")
        .expect("explicit real worker")
        .into()
}
async fn inspect(root: &Path) {
    let source = FolderProject::open(root).await.unwrap();
    let original = source.capture().await.unwrap();
    let project = original.project();
    assert_eq!(project.revisions.len(), 1);
    let revision = &project.revisions[0];
    assert_eq!(project.current_revision.as_ref(), Some(&revision.id));
    let file = project
        .files
        .iter()
        .find(|file| Some(&file.name) == revision.source.as_ref())
        .unwrap();
    let bytes = source.read_asset(&original, file).await.unwrap();
    let check = tempfile::tempdir().unwrap();
    std::fs::write(check.path().join("source.step"), bytes).unwrap();
    let report = crate::native::inspection::inspect_with_executable(
        check.path(),
        Arc::new(AtomicBool::new(false)),
        &worker(),
    )
    .await
    .unwrap();
    assert!((report.volume_mm3 - 8000.).abs() < 1e-6);
    for (actual, expected) in report.bounds_mm.into_iter().zip([40., 20., 10.]) {
        assert!((actual - expected).abs() < 1e-6);
    }
    source.recheck(&original).await.unwrap();
}

#[tokio::test]
#[ignore = "requires explicitly built real CAD worker"]
async fn real_worker_host_api_roundtrip() {
    let root = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let initial: Project = serde_json::from_value(serde_json::json!({
        "schemaVersion":1,"id":uuid::Uuid::new_v4().to_string(),"name":"Public host CAD fixture",
        "units":"mm","agent":"codex","pinned":false,"createdAt":"2026-11-03T00:00:00Z",
        "updatedAt":"2026-11-03T00:00:00Z","currentRevision":null,"messages":[],
        "exports":[],"files":[],"revisions":[]
    }))
    .unwrap();
    assert!(matches!(
        FolderProject::create(root.path(), initial, vec![], None)
            .await
            .unwrap(),
        Publication::Committed(_)
    ));
    let source = FolderProject::open(root.path()).await.unwrap();
    let original = source.capture().await.unwrap();
    let approval = source.approve_write(&original).await.unwrap();
    let tasks = Mutex::new(HashMap::new());
    let gate = CadTaskGate::default();
    let executable = worker();
    let runtime = || ModelRuntime {
        tasks: &tasks,
        gate: &gate,
        runtime_root: work.path(),
        transient_root: work.path(),
        executable: Some(&executable),
    };
    let change = source
        .apply_model(
            &original,
            Some(approval),
            ModelIntent {
                program: program(40.),
                prompt: "Explicit real CAD command".into(),
            },
            runtime(),
        )
        .await
        .unwrap();
    let receipt = match change {
        ModelChange::Published(Publication::Committed(saved)) => saved,
        _ => panic!("expected actual confirmed model publication"),
    };
    assert_ne!(receipt.commit_id(), original.commit_id());
    assert!(source.recheck(&original).await.is_err());
    let latest = source.capture().await.unwrap();
    let saved_program = latest
        .project()
        .revisions
        .last()
        .unwrap()
        .program
        .clone()
        .unwrap();
    assert!(matches!(
        source
            .apply_model(
                &latest,
                None,
                ModelIntent {
                    program: saved_program,
                    prompt: "No-op needs no grant".into()
                },
                runtime()
            )
            .await
            .unwrap(),
        ModelChange::NoOp(_)
    ));
    assert!(source
        .apply_model(
            &latest,
            None,
            ModelIntent {
                program: program(80.),
                prompt: "Unapproved change".into()
            },
            runtime()
        )
        .await
        .is_err());
    source.recheck(&latest).await.unwrap();
    assert!(tasks.lock().await.is_empty());
    assert!(gate.status().unwrap().is_empty());
    source.close();
    assert!(source.recheck(&latest).await.is_err());
    drop(source);
    inspect(root.path()).await;
    let nonce = uuid::Uuid::new_v4().to_string();
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "project_repository::sqlite_folder::api::worker_tests::host_api_child",
            "--nocapture",
        ])
        .env("FORMA_FOLDER_TEST_WORKER", &executable)
        .env("FORMA_HOST_API_TARGET", root.path())
        .env("FORMA_HOST_API_NONCE", &nonce)
        .output()
        .unwrap();
    assert!(
        child.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&child.stdout),
        String::from_utf8_lossy(&child.stderr)
    );
    assert!(String::from_utf8_lossy(&child.stdout)
        .lines()
        .any(|line| line == format!("HOST_API_REOPEN_OK:{nonce}")));
}

#[tokio::test]
#[ignore = "exact host API independent-process probe"]
async fn host_api_child() {
    let root =
        std::path::PathBuf::from(std::env::var_os("FORMA_HOST_API_TARGET").expect("parent target"));
    inspect(&root).await;
    println!(
        "\nHOST_API_REOPEN_OK:{}",
        std::env::var("FORMA_HOST_API_NONCE").unwrap()
    );
}
