//! Standalone folder host tests. Synthetic policy fixtures are separate from
//! the explicitly requested real-worker geometry roundtrip below.
use super::*;
use crate::{
    cad_tasks::CadTaskGate,
    core::AppError,
    modeling::repository_apply::{self, ApplyRuntime, RepositoryApply},
    project_repository::sqlite_folder::creation,
};
use std::{collections::HashMap, path::Path, sync::atomic::Ordering, time::Duration};
use tokio::sync::Mutex as AsyncMutex;

const SOURCE: &str = "result = cq.Workplane('XY').box(40,20,10)";

fn project(with_revision: bool) -> Project {
    let mut project: Project = serde_json::from_value(serde_json::json!({
        "schemaVersion":1,"id":uuid::Uuid::new_v4().to_string(),"name":"folder apply fixture",
        "units":"mm","agent":"codex","pinned":false,"createdAt":"2026-11-02T00:00:00Z",
        "updatedAt":"2026-11-02T00:00:00Z","currentRevision":null,"messages":[],
        "exports":[],"files":[],"revisions":[]
    }))
    .unwrap();
    if with_revision {
        let revision = uuid::Uuid::new_v4().to_string();
        project.current_revision = Some(revision.clone());
        project.revisions.push(serde_json::from_value(serde_json::json!({
            "id":revision,"parent":null,"createdAt":"2026-11-02T00:00:00Z","prompt":"fixture",
            "program":SOURCE,"parameters":{"kind":"box","width":40,"depth":20,"height":10,"thickness":2,"holeDiameter":0,"holes":0}
        })).unwrap());
    }
    project
}

async fn fixture(root: &Path, with_revision: bool) -> FolderSession {
    assert!(matches!(
        creation::create(root, project(with_revision), vec![], None)
            .await
            .unwrap(),
        format::CommitOutcome::Committed(_)
    ));
    FolderSession::open(root).await.unwrap()
}

fn runtime<'a>(root: &'a Path, executable: Option<&'a Path>) -> BuildRuntime<'a> {
    BuildRuntime {
        runtime_root: root,
        transient_root: root,
        executable,
    }
}
fn request(project: &Project, program: &str) -> ApplyRequest {
    ApplyRequest::new(
        project.id.clone(),
        program.into(),
        "folder CAD edit".into(),
        project.current_revision.clone(),
    )
}
fn assert_send<T: Send>(_: T) {}

#[tokio::test]
async fn folder_no_op_has_no_grant_or_task_and_future_is_send() {
    let root = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let session = fixture(root.path(), true).await;
    let original = session.capture().await.unwrap();
    let before = std::fs::read(root.path().join("project.sqlite")).unwrap();
    let host = FolderApplyHost::new(&session, runtime(work.path(), None), None);
    let tasks = AsyncMutex::new(HashMap::new());
    let gate = CadTaskGate::default();
    assert_send(repository_apply::apply(
        ApplyRuntime::new(&tasks, &gate),
        &host,
        request(original.project(), SOURCE),
    ));
    assert!(matches!(
        repository_apply::apply(
            ApplyRuntime::new(&tasks, &gate),
            &host,
            request(original.project(), SOURCE)
        )
        .await
        .unwrap(),
        RepositoryApply::NoOp(_)
    ));
    assert!(tasks.lock().await.is_empty());
    assert!(gate.status().unwrap().is_empty());
    assert_eq!(
        before,
        std::fs::read(root.path().join("project.sqlite")).unwrap()
    );
    assert_eq!(std::fs::read_dir(work.path()).unwrap().count(), 0);
    assert!(
        matches!(repository_apply::apply(ApplyRuntime::new(&tasks, &gate), &host, request(original.project(), "result = cq.Workplane('XY').box(80,20,10)")).await, Err(AppError::Invalid(message)) if message == "PERMISSION_REQUIRED")
    );
}

#[tokio::test]
async fn host_grant_is_consumed_once_and_bound_to_opening_not_project_id() {
    let root = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let session = fixture(root.path(), true).await;
    let original = session.capture().await.unwrap();
    let second = FolderSession::open(root.path()).await.unwrap();
    let grant = FolderAuthorization::authorize_modeling(&session, &original)
        .await
        .unwrap();
    let wrong = FolderApplyHost::new(&second, runtime(work.path(), None), Some(grant));
    assert_ne!(wrong.task_key(), session.opening.task_id);
    assert_ne!(wrong.task_key(), original.project().id);
    assert!(wrong.authorize().await.is_err());
    let grant = FolderAuthorization::authorize_modeling(&session, &original)
        .await
        .unwrap();
    let host = FolderApplyHost::new(&session, runtime(work.path(), None), Some(grant));
    let consumed = host.authorize().await.unwrap();
    consumed.check(&session).unwrap();
    assert!(host.authorize().await.is_err());
    session.close();
    assert!(consumed.check(&session).is_err());
    assert!(FolderAuthorization::authorize_modeling(&session, &original)
        .await
        .is_err());
}

async fn wait_for_task(tasks: &AsyncMutex<HashMap<String, Arc<AtomicBool>>>, key: &str) {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if tasks.lock().await.contains_key(key) {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn saved_state_changed_after_approval_cannot_be_recaptured_as_the_approved_command() {
    let root = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let session = fixture(root.path(), true).await;
    let original = session.capture().await.unwrap();
    let grant = FolderAuthorization::authorize_modeling(&session, &original)
        .await
        .unwrap();
    let mut renamed = original.project().clone();
    renamed.name = "Saved by another operation after approval".into();
    assert!(matches!(
        session
            .commit(&original, renamed.clone(), vec![], None)
            .await
            .unwrap(),
        format::CommitOutcome::Committed(_)
    ));
    let before = std::fs::read(root.path().join("project.sqlite")).unwrap();
    let host = FolderApplyHost::new(&session, runtime(work.path(), None), Some(grant));
    let tasks = AsyncMutex::new(HashMap::new());
    let gate = CadTaskGate::default();
    assert!(repository_apply::apply(
        ApplyRuntime::new(&tasks, &gate),
        &host,
        request(
            original.project(),
            "result = cq.Workplane('XY').box(80,20,10)"
        ),
    )
    .await
    .is_err());
    assert_eq!(
        before,
        std::fs::read(root.path().join("project.sqlite")).unwrap()
    );
    assert_eq!(
        serde_json::to_value(session.capture().await.unwrap().project()).unwrap(),
        serde_json::to_value(&renamed).unwrap()
    );
    assert!(tasks.lock().await.is_empty());
    assert!(gate.status().unwrap().is_empty());
    assert_eq!(std::fs::read_dir(work.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn queued_cancellation_and_close_do_not_enter_worker_or_leave_registration() {
    for close in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let work = tempfile::tempdir().unwrap();
        let session = fixture(root.path(), true).await;
        let original = session.capture().await.unwrap();
        let before = std::fs::read(root.path().join("project.sqlite")).unwrap();
        let grant = FolderAuthorization::authorize_modeling(&session, &original)
            .await
            .unwrap();
        let host = FolderApplyHost::new(&session, runtime(work.path(), None), Some(grant));
        let tasks = AsyncMutex::new(HashMap::new());
        let gate = CadTaskGate::default();
        let occupied = gate
            .acquire(
                &uuid::Uuid::new_v4().to_string(),
                "test_barrier",
                Arc::new(AtomicBool::new(false)),
            )
            .await
            .unwrap();
        let mutation = async {
            wait_for_task(&tasks, host.task_key()).await;
            if close {
                session.close();
            } else {
                tasks
                    .lock()
                    .await
                    .get(host.task_key())
                    .unwrap()
                    .store(true, Ordering::Relaxed);
            }
            drop(occupied);
        };
        let (outcome, ()) = tokio::join!(
            repository_apply::apply(
                ApplyRuntime::new(&tasks, &gate),
                &host,
                request(
                    original.project(),
                    "result = cq.Workplane('XY').box(80,20,10)"
                )
            ),
            mutation
        );
        assert!(outcome.is_err());
        assert!(tasks.lock().await.is_empty());
        assert!(gate.status().unwrap().is_empty());
        assert_eq!(
            before,
            std::fs::read(root.path().join("project.sqlite")).unwrap()
        );
        assert_eq!(std::fs::read_dir(work.path()).unwrap().count(), 0);
    }
}

#[cfg(feature = "native-worker-client")]
fn box_program(width: f64) -> String {
    serde_json::json!({"schemaVersion":2,"revisionId":"folder_candidate","parameters":[],"features":[
        {"id":"profile","name":"Profile","operation":{"type":"rectangle","width":{"kind":"literal","mm":width},"depth":{"kind":"literal","mm":20}}},
        {"id":"pad","name":"Pad","operation":{"type":"extrude","sketchId":"profile","distance":{"kind":"literal","mm":10}}}
    ],"bodies":[{"id":"body","name":"Body","sourceFeatureId":"pad"}]}).to_string()
}

#[cfg(feature = "native-worker-client")]
#[test]
fn repeated_imports_select_first_matching_step_descriptor_once() {
    let mut project = project(true);
    let hash = crate::artifacts::digest(b"synthetic import bytes");
    for name in ["first.step", "alias.stp", "not-step.bin"] {
        project.files.push(ProjectFile {
            name: name.into(),
            size: 22,
            kind: "model".into(),
            data: None,
            sha256: Some(hash.clone()),
        });
    }
    let program = serde_json::json!({"schemaVersion":2,"revisionId":"folder_candidate","parameters":[],
        "features":[
            {"id":"one","name":"One","operation":{"type":"importStep","assetId":format!("step_{hash}"),"sha256":hash}},
            {"id":"two","name":"Two","operation":{"type":"importStep","assetId":format!("step_{hash}"),"sha256":hash}}
        ],"bodies":[{"id":"body_one","name":"One","sourceFeatureId":"one"},{"id":"body_two","name":"Two","sourceFeatureId":"two"}]
    }).to_string();
    let request = request(&project, &program);
    let crate::modeling::prepared_request::PreparedRequest::Apply(prepared) =
        crate::modeling::prepared_request::prepare(&project, &request).unwrap()
    else {
        panic!("real request")
    };
    let selected = requested_assets(&project, &request, &prepared).unwrap();
    assert_eq!(selected, vec![project.files[0].clone()]);
}

#[cfg(feature = "native-worker-client")]
async fn check_geometry(session: &FolderSession, expected_width: f64, worker: &Path) {
    let snapshot = session.capture().await.unwrap();
    let revision = snapshot
        .project()
        .revisions
        .iter()
        .find(|revision| Some(&revision.id) == snapshot.project().current_revision.as_ref())
        .unwrap();
    let descriptor = snapshot
        .project()
        .files
        .iter()
        .find(|file| Some(&file.name) == revision.source.as_ref())
        .unwrap();
    let bytes = session.read_asset(&snapshot, descriptor).await.unwrap();
    let inspection = tempfile::tempdir().unwrap();
    std::fs::write(inspection.path().join("source.step"), bytes).unwrap();
    let properties = crate::native::inspection::inspect_with_executable(
        inspection.path(),
        Arc::new(AtomicBool::new(false)),
        worker,
    )
    .await
    .unwrap();
    assert!((properties.volume_mm3 - expected_width * 20.0 * 10.0).abs() < 1e-6);
    for (actual, expected) in properties
        .bounds_mm
        .into_iter()
        .zip([expected_width, 20.0, 10.0])
    {
        assert!((actual - expected).abs() < 1e-6);
    }
    session.recheck(&snapshot).await.unwrap();
}

#[cfg(feature = "native-worker-client")]
#[tokio::test]
#[ignore = "requires explicitly built real CAD worker via FORMA_FOLDER_TEST_WORKER"]
async fn real_worker_folder_roundtrip() {
    let worker = std::path::PathBuf::from(
        std::env::var_os("FORMA_FOLDER_TEST_WORKER")
            .expect("Set FORMA_FOLDER_TEST_WORKER to the real built CAD worker"),
    );
    assert!(worker.is_file());
    let root = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let session = fixture(root.path(), false).await;
    let tasks = AsyncMutex::new(HashMap::new());
    let gate = CadTaskGate::default();
    for width in [40.0, 80.0] {
        let original = session.capture().await.unwrap();
        let grant = FolderAuthorization::authorize_modeling(&session, &original)
            .await
            .unwrap();
        let host = FolderApplyHost::new(&session, runtime(work.path(), Some(&worker)), Some(grant));
        let outcome = repository_apply::apply(
            ApplyRuntime::new(&tasks, &gate),
            &host,
            request(original.project(), &box_program(width)),
        )
        .await
        .unwrap();
        assert!(matches!(
            outcome,
            RepositoryApply::Published(format::CommitOutcome::Committed(_))
        ));
        check_geometry(&session, width, &worker).await;
        assert!(tasks.lock().await.is_empty());
    }
    let edited = session.capture().await.unwrap();
    assert!(matches!(
        session
            .apply_history(&edited, crate::project_history::Direction::Undo)
            .await
            .unwrap(),
        format::CommitOutcome::Committed(_)
    ));
    check_geometry(&session, 40.0, &worker).await;
    session.close();
    drop(session);
    let nonce = uuid::Uuid::new_v4().to_string();
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "project_repository::sqlite_folder::session::apply::tests::real_worker_folder_child",
            "--nocapture",
        ])
        .env("FORMA_FOLDER_TEST_WORKER", &worker)
        .env("FORMA_FOLDER_ROUNDTRIP_ROOT", root.path())
        .env("FORMA_FOLDER_ROUNDTRIP_NONCE", &nonce)
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
        .any(|line| line == format!("FOLDER_REAL_CAD_REOPEN_OK:{nonce}")));
    let reopened = FolderSession::open(root.path()).await.unwrap();
    let undone = reopened.capture().await.unwrap();
    assert!(undone.saved.history_state().can_redo());
    assert!(matches!(
        reopened
            .apply_history(&undone, crate::project_history::Direction::Redo)
            .await
            .unwrap(),
        format::CommitOutcome::Committed(_)
    ));
    check_geometry(&reopened, 80.0, &worker).await;
}

#[cfg(feature = "native-worker-client")]
#[tokio::test]
#[ignore = "owned fresh-process real CAD geometry probe"]
async fn real_worker_folder_child() {
    let root = std::path::PathBuf::from(
        std::env::var_os("FORMA_FOLDER_ROUNDTRIP_ROOT").expect("parent supplied folder"),
    );
    let worker = std::path::PathBuf::from(
        std::env::var_os("FORMA_FOLDER_TEST_WORKER").expect("parent supplied real worker"),
    );
    let session = FolderSession::open(&root).await.unwrap();
    let snapshot = session.capture().await.unwrap();
    assert!(snapshot.saved.history_state().can_redo());
    // Undo materializes a new immutable revision, retaining both authored models.
    assert_eq!(snapshot.project().revisions.len(), 3);
    check_geometry(&session, 40.0, &worker).await;
    println!(
        "\nFOLDER_REAL_CAD_REOPEN_OK:{}",
        std::env::var("FORMA_FOLDER_ROUNDTRIP_NONCE").unwrap()
    );
}
