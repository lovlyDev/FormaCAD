#![cfg(feature = "native-occt")]
use forma_core::{cad_tasks::CadTaskGate, native::worker};
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::sync::oneshot;

fn source() -> String {
    include_str!("../../../../docs/fixtures/parameterized-disc.cad.json").into()
}
async fn queued(gate: &CadTaskGate, project_id: &str) {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if gate
                .status()
                .unwrap()
                .iter()
                .any(|task| task.project_id == project_id && task.state == "queued")
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}
async fn run(
    gate: Arc<CadTaskGate>,
    project: String,
    cwd: PathBuf,
    active: Arc<AtomicUsize>,
    wait: Option<oneshot::Receiver<()>>,
    started: oneshot::Sender<()>,
) {
    let cancel = Arc::new(AtomicBool::new(false));
    let _permit = gate
        .acquire(&project, "model_build", cancel.clone())
        .await
        .unwrap();
    assert_eq!(
        active.fetch_add(1, Ordering::SeqCst),
        0,
        "two CAD workers overlapped"
    );
    std::fs::create_dir(&cwd).unwrap();
    assert!(worker::build_with_executable(
        &source(),
        &cwd,
        cancel,
        Path::new(env!("CARGO_BIN_EXE_forma-cad-worker"))
    )
    .await
    .unwrap());
    assert!(std::fs::metadata(cwd.join("model.step")).unwrap().len() > 100);
    assert!(std::fs::metadata(cwd.join("preview.glb")).unwrap().len() > 100);
    started.send(()).unwrap();
    if let Some(wait) = wait {
        wait.await.unwrap();
    }
    assert_eq!(active.fetch_sub(1, Ordering::SeqCst), 1);
}

#[tokio::test]
async fn two_project_jobs_execute_real_workers_sequentially() {
    let root = tempfile::tempdir().unwrap();
    let gate = Arc::new(CadTaskGate::default());
    let active = Arc::new(AtomicUsize::new(0));
    let first = uuid::Uuid::new_v4().to_string();
    let second = uuid::Uuid::new_v4().to_string();
    let (release, wait) = oneshot::channel();
    let (done_first, finished_first) = oneshot::channel();
    let one = tokio::spawn(run(
        gate.clone(),
        first.clone(),
        root.path().join("one"),
        active.clone(),
        Some(wait),
        done_first,
    ));
    finished_first.await.unwrap();
    let (done_second, mut finished_second) = oneshot::channel();
    let two = tokio::spawn(run(
        gate.clone(),
        second.clone(),
        root.path().join("two"),
        active.clone(),
        None,
        done_second,
    ));
    queued(&gate, &second).await;
    assert!(!root.path().join("two").exists());
    assert!(finished_second.try_recv().is_err());
    assert_eq!(
        gate.status()
            .unwrap()
            .iter()
            .filter(|task| task.state == "running")
            .count(),
        1
    );
    release.send(()).unwrap();
    one.await.unwrap();
    two.await.unwrap();
    finished_second.await.unwrap();
    assert!(gate.status().unwrap().is_empty());
    assert_eq!(active.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn cancelled_waiting_project_never_spawns_worker_or_changes_saved_state() {
    let root = tempfile::tempdir().unwrap();
    let sentinel = root.path().join("saved-project.json");
    std::fs::write(&sentinel, b"immutable committed snapshot").unwrap();
    let gate = Arc::new(CadTaskGate::default());
    let first = uuid::Uuid::new_v4().to_string();
    let second = uuid::Uuid::new_v4().to_string();
    let permit = gate
        .acquire(&first, "model_build", Arc::new(AtomicBool::new(false)))
        .await
        .unwrap();
    let waiting_gate = gate.clone();
    let waiting_id = second.clone();
    let output = root.path().join("never-started");
    let waiting_output = output.clone();
    let waiting = tokio::spawn(async move {
        let cancel = Arc::new(AtomicBool::new(false));
        let _permit = waiting_gate
            .acquire(&waiting_id, "step_export", cancel.clone())
            .await?;
        std::fs::create_dir(&waiting_output)?;
        worker::build_with_executable(
            &source(),
            &waiting_output,
            cancel,
            Path::new(env!("CARGO_BIN_EXE_forma-cad-worker")),
        )
        .await
    });
    queued(&gate, &second).await;
    gate.cancel_project(&second).unwrap();
    let result = tokio::time::timeout(Duration::from_secs(2), waiting)
        .await
        .unwrap()
        .unwrap();
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("CAD_TASK_CANCELLED"));
    assert!(!output.exists());
    assert_eq!(
        std::fs::read(&sentinel).unwrap(),
        b"immutable committed snapshot"
    );
    assert_eq!(gate.status().unwrap().len(), 1);
    drop(permit);
    assert!(gate.status().unwrap().is_empty());
}
