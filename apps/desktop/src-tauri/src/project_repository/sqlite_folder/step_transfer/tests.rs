//! Standalone policy checks; real STEP geometry is explicitly run in native CI.
use super::*;
use crate::cad_tasks::CadTaskGate;
use std::{collections::HashMap, path::Path, sync::atomic::Ordering, time::Duration};
use tokio::sync::Mutex as AsyncMutex;

fn intent() -> TransferIntent {
    TransferIntent {
        name: "Transferred part".into(),
        agent: "codex".into(),
        prompt: "Explicit selected STEP import".into(),
    }
}
fn runtime(root: &Path) -> BuildRuntime<'_> {
    BuildRuntime {
        runtime_root: root,
        transient_root: root,
        executable: None,
    }
}
fn assert_send<T: Send>(_: T) {}

#[tokio::test]
async fn closed_or_other_source_grant_cannot_touch_destination_and_future_is_send() {
    let old = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let path = old.path().join("last.step");
    std::fs::write(&path, b"synthetic source bytes").unwrap();
    let source = SelectedStep::open(&path).unwrap();
    let original = source.capture().unwrap();
    let grant = StepTransferGrant::approve(&source, &original, target.path()).unwrap();
    let other = SelectedStep::open(&path).unwrap();
    let other_snapshot = other.capture().unwrap();
    assert!(grant.check(&other, &other_snapshot).is_err());
    source.close();
    let tasks = AsyncMutex::new(HashMap::new());
    let gate = CadTaskGate::default();
    assert_send(transfer(
        &source,
        &original,
        grant,
        intent(),
        ApplyRuntime::new(&tasks, &gate),
        runtime(work.path()),
    ));
    // A separate explicit grant on the other opening still cannot survive close.
    let grant = StepTransferGrant::approve(&other, &other_snapshot, target.path()).unwrap();
    other.close();
    assert!(transfer(
        &other,
        &other_snapshot,
        grant,
        intent(),
        ApplyRuntime::new(&tasks, &gate),
        runtime(work.path())
    )
    .await
    .is_err());
    assert_eq!(std::fs::read_dir(target.path()).unwrap().count(), 0);
    assert!(tasks.lock().await.is_empty());
    assert_eq!(std::fs::read(path).unwrap(), b"synthetic source bytes");
}

#[tokio::test]
async fn queued_close_or_cancellation_never_runs_worker_or_creates_project() {
    for close in [false, true] {
        let old = tempfile::tempdir().unwrap();
        let target = tempfile::tempdir().unwrap();
        let work = tempfile::tempdir().unwrap();
        let path = old.path().join("last.step");
        std::fs::write(&path, b"synthetic selected bytes").unwrap();
        let source = SelectedStep::open(&path).unwrap();
        let original = source.capture().unwrap();
        let grant = StepTransferGrant::approve(&source, &original, target.path()).unwrap();
        let tasks: AsyncMutex<HashMap<String, Arc<AtomicBool>>> = AsyncMutex::new(HashMap::new());
        let gate = CadTaskGate::default();
        let occupied = gate
            .acquire(
                &uuid::Uuid::new_v4().to_string(),
                "test_barrier",
                Arc::new(AtomicBool::new(false)),
            )
            .await
            .unwrap();
        let change = async {
            tokio::time::timeout(Duration::from_secs(5), async {
                loop {
                    if tasks.lock().await.contains_key(original.task_key()) {
                        break;
                    }
                    tokio::task::yield_now().await;
                }
            })
            .await
            .unwrap();
            if close {
                source.close();
            } else {
                tasks
                    .lock()
                    .await
                    .get(original.task_key())
                    .unwrap()
                    .store(true, Ordering::Relaxed);
            }
            drop(occupied);
        };
        let (result, ()) = tokio::join!(
            transfer(
                &source,
                &original,
                grant,
                intent(),
                ApplyRuntime::new(&tasks, &gate),
                runtime(work.path())
            ),
            change
        );
        assert!(result.is_err());
        assert!(tasks.lock().await.is_empty());
        assert!(gate.status().unwrap().is_empty());
        assert_eq!(std::fs::read_dir(target.path()).unwrap().count(), 0);
        assert_eq!(std::fs::read_dir(work.path()).unwrap().count(), 0);
        assert_eq!(std::fs::read(path).unwrap(), b"synthetic selected bytes");
    }
}

#[tokio::test]
async fn nonempty_target_and_invalid_project_intent_are_refused_before_publication() {
    let old = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let path = old.path().join("last.step");
    std::fs::write(&path, b"synthetic input").unwrap();
    let source = SelectedStep::open(&path).unwrap();
    let original = source.capture().unwrap();
    std::fs::write(target.path().join("keep.txt"), b"foreign bytes").unwrap();
    assert!(StepTransferGrant::approve(&source, &original, target.path()).is_err());
    assert_eq!(
        std::fs::read(target.path().join("keep.txt")).unwrap(),
        b"foreign bytes"
    );
    let empty = tempfile::tempdir().unwrap();
    let grant = StepTransferGrant::approve(&source, &original, empty.path()).unwrap();
    let tasks = AsyncMutex::new(HashMap::new());
    let gate = CadTaskGate::default();
    let mut invalid = intent();
    invalid.name.clear();
    assert!(transfer(
        &source,
        &original,
        grant,
        invalid,
        ApplyRuntime::new(&tasks, &gate),
        runtime(work.path())
    )
    .await
    .is_err());
    assert_eq!(std::fs::read_dir(empty.path()).unwrap().count(), 0);
    source.recheck(&original).unwrap();
}

async fn inspect_saved(root: &Path, worker: &Path) {
    let session = super::super::session::FolderSession::open(root)
        .await
        .unwrap();
    let saved = session.capture().await.unwrap();
    assert_eq!(saved.project().revisions.len(), 1);
    let revision = &saved.project().revisions[0];
    assert!(revision.parent.is_none());
    let program: serde_json::Value =
        serde_json::from_str(revision.program.as_ref().unwrap()).unwrap();
    assert_eq!(program["features"].as_array().unwrap().len(), 1);
    assert_eq!(program["features"][0]["operation"]["type"], "importStep");
    let file = saved
        .project()
        .files
        .iter()
        .find(|file| Some(&file.name) == revision.source.as_ref())
        .unwrap();
    let bytes = session.read_asset(&saved, file).await.unwrap();
    let check = tempfile::tempdir().unwrap();
    std::fs::write(check.path().join("source.step"), bytes).unwrap();
    let properties = crate::native::inspection::inspect_with_executable(
        check.path(),
        Arc::new(AtomicBool::new(false)),
        worker,
    )
    .await
    .unwrap();
    assert!((properties.volume_mm3 - 8000.0).abs() < 1e-6);
    for (actual, expected) in properties.bounds_mm.into_iter().zip([40.0, 20.0, 10.0]) {
        assert!((actual - expected).abs() < 1e-6);
    }
    session.recheck(&saved).await.unwrap();
}

#[tokio::test]
#[ignore = "requires explicitly built real worker via FORMA_FOLDER_TEST_WORKER"]
async fn real_worker_step_transfer_roundtrip() {
    let worker = std::path::PathBuf::from(
        std::env::var_os("FORMA_FOLDER_TEST_WORKER").expect("explicit real worker"),
    );
    assert!(worker.is_file());
    let generated = tempfile::tempdir().unwrap();
    let old = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let program=serde_json::json!({"schemaVersion":2,"revisionId":"source","parameters":[],"features":[
        {"id":"profile","name":"Profile","operation":{"type":"rectangle","width":{"kind":"literal","mm":40},"depth":{"kind":"literal","mm":20}}},
        {"id":"pad","name":"Pad","operation":{"type":"extrude","sketchId":"profile","distance":{"kind":"literal","mm":10}}}
    ],"bodies":[{"id":"body","name":"Body","sourceFeatureId":"pad"}]}).to_string();
    assert!(crate::native::worker::build_with_executable(
        &program,
        generated.path(),
        Arc::new(AtomicBool::new(false)),
        &worker
    )
    .await
    .unwrap());
    let bytes = std::fs::read(generated.path().join("model.step")).unwrap();
    let path = old.path().join("last.step");
    std::fs::write(&path, &bytes).unwrap();
    // Synthetic directory marker is not represented as an actual 1.1 project fixture.
    let old_id = uuid::Uuid::new_v4().to_string();
    let marker = old.path().join("old-project-sentinel");
    std::fs::write(&marker, old_id.as_bytes()).unwrap();
    let source = SelectedStep::open(&path).unwrap();
    let original = source.capture().unwrap();
    let grant = StepTransferGrant::approve(&source, &original, target.path()).unwrap();
    let tasks = AsyncMutex::new(HashMap::new());
    let gate = CadTaskGate::default();
    let outcome = transfer(
        &source,
        &original,
        grant,
        intent(),
        ApplyRuntime::new(&tasks, &gate),
        BuildRuntime {
            executable: Some(&worker),
            ..runtime(work.path())
        },
    )
    .await
    .unwrap();
    let saved = match outcome {
        format::CommitOutcome::Committed(saved) => saved,
        other => panic!("transfer not confirmed: {other:?}"),
    };
    assert_ne!(saved.project().id, old_id);
    assert_ne!(saved.project().id, original.task_key());
    assert_eq!(saved.project().revisions.len(), 1);
    assert_eq!(saved.expectation().sequence(), 1);
    assert!(tasks.lock().await.is_empty());
    assert!(gate.status().unwrap().is_empty());
    source.recheck(&original).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert_eq!(std::fs::read(&marker).unwrap(), old_id.as_bytes());
    assert_eq!(std::fs::read_dir(old.path()).unwrap().count(), 2);
    inspect_saved(target.path(), &worker).await;
    let nonce = uuid::Uuid::new_v4().to_string();
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "project_repository::sqlite_folder::step_transfer::tests::step_transfer_child",
            "--nocapture",
        ])
        .env("FORMA_FOLDER_TEST_WORKER", &worker)
        .env("FORMA_STEP_TRANSFER_TARGET", target.path())
        .env("FORMA_STEP_TRANSFER_NONCE", &nonce)
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
        .any(|line| line == format!("STEP_TRANSFER_REOPEN_OK:{nonce}")));
    // Bad geometry must fail before the target's first SQL/CAS publication.
    let bad = old.path().join("invalid.step");
    std::fs::write(&bad, b"not STEP geometry").unwrap();
    let invalid_source = SelectedStep::open(&bad).unwrap();
    let invalid_snapshot = invalid_source.capture().unwrap();
    let refused = tempfile::tempdir().unwrap();
    let grant =
        StepTransferGrant::approve(&invalid_source, &invalid_snapshot, refused.path()).unwrap();
    assert!(transfer(
        &invalid_source,
        &invalid_snapshot,
        grant,
        intent(),
        ApplyRuntime::new(&tasks, &gate),
        BuildRuntime {
            executable: Some(&worker),
            ..runtime(work.path())
        }
    )
    .await
    .is_err());
    assert_eq!(std::fs::read_dir(refused.path()).unwrap().count(), 0);
    assert!(tasks.lock().await.is_empty());
}

#[tokio::test]
#[ignore = "exact selected STEP independent-process probe"]
async fn step_transfer_child() {
    let target = std::path::PathBuf::from(
        std::env::var_os("FORMA_STEP_TRANSFER_TARGET").expect("parent target"),
    );
    let worker = std::path::PathBuf::from(
        std::env::var_os("FORMA_FOLDER_TEST_WORKER").expect("parent worker"),
    );
    inspect_saved(&target, &worker).await;
    println!(
        "\nSTEP_TRANSFER_REOPEN_OK:{}",
        std::env::var("FORMA_STEP_TRANSFER_NONCE").unwrap()
    );
}
