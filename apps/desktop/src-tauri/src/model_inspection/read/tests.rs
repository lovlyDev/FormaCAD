use super::*;
use crate::project_repository::legacy::read_test_fixture as fixture;
fn worker() -> PathBuf {
    option_env!("CARGO_BIN_EXE_forma-cad-worker")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::current_exe()
                .unwrap()
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join(format!("forma-cad-worker{}", std::env::consts::EXE_SUFFIX))
        })
}
async fn saved_step(state: &AppState) -> crate::models::Project {
    let mut project = fixture::project(state, 4).await;
    // Generate the genuine STEP in an isolated process, as the desktop does.
    // Worker exit closes its native IO before the host reads and removes staging.
    let staging = tempfile::tempdir().unwrap();
    let document = serde_json::json!({
        "schemaVersion":2,"revisionId":"fixture_box","parameters":[],
        "features":[
            {"id":"profile","name":"Profile","operation":{"type":"rectangle","width":{"kind":"literal","mm":40.0},"depth":{"kind":"literal","mm":20.0}}},
            {"id":"pad","name":"Pad","operation":{"type":"extrude","sketchId":"profile","distance":{"kind":"literal","mm":10.0}}}
        ],
        "bodies":[{"id":"body","name":"Box","sourceFeatureId":"pad"}]
    });
    assert!(crate::native::worker::build_with_executable(
        &document.to_string(),
        staging.path(),
        Arc::new(AtomicBool::new(false)),
        &worker()
    )
    .await
    .unwrap());
    let bytes = std::fs::read(staging.path().join("model.step")).unwrap();
    staging
        .close()
        .expect("worker STEP staging must be fully releasable after exit");
    let file = &mut project.files[0];
    file.size = bytes.len() as u64;
    file.data = Some(crate::artifacts::data_url(&file.name, &bytes));
    file.sha256 = Some(crate::artifacts::digest(&bytes));
    fixture::replace(state, &project, false).await;
    project
}
#[tokio::test]
async fn pending_journal_does_not_recover_or_stage_step() {
    let root = tempfile::tempdir().unwrap();
    let state = fixture::state(root.path()).await;
    let project = fixture::project(&state, 4).await;
    fixture::pending(&state, &project.id).await;
    let before = fixture::logical(&state, &project.id).await;
    let files = fixture::files(root.path());
    let result = run(
        &state,
        &project.id,
        project.current_revision.as_deref().unwrap(),
        |_, _| async { panic!("must not inspect") },
    )
    .await;
    assert!(
        matches!(result,Err(AppError::Invalid(ref v)) if v == "PROJECT_SNAPSHOT_RECOVERY_REQUIRED")
    );
    assert_eq!(fixture::logical(&state, &project.id).await, before);
    assert_eq!(fixture::files(root.path()), files);
}
#[tokio::test]
async fn genuine_imported_step_old_revision_read_only_and_worker_return_race() {
    let root = tempfile::tempdir().unwrap();
    let state = fixture::state(root.path()).await;
    let original = saved_step(&state).await;
    let project = fixture::next_head(&state, &original).await;
    let old_revision = original.current_revision.as_deref().unwrap();
    let before = fixture::logical(&state, &project.id).await;
    let files = fixture::files(root.path());
    let properties = run(&state, &project.id, old_revision, |p, c| async move {
        crate::native::inspection::inspect_with_executable(&p, c, &worker()).await
    })
    .await
    .unwrap()
    .unwrap();
    assert!((properties.volume_mm3 - 8000.).abs() < 1e-6);
    for (actual, expected) in properties.bounds_mm.into_iter().zip([40., 20., 10.]) {
        assert!((actual - expected).abs() < 1e-6);
    }
    assert_eq!(fixture::logical(&state, &project.id).await, before);
    assert_eq!(fixture::files(root.path()), files);
    let result = run(&state, &project.id, old_revision, |p, c| {
        let state = &state;
        let project = &project;
        async move {
            let properties =
                crate::native::inspection::inspect_with_executable(&p, c, &worker()).await?;
            fixture::next_head(state, project).await;
            Ok(properties)
        }
    })
    .await;
    assert!(matches!(result,Err(AppError::Invalid(ref v)) if v == "PROJECT_SNAPSHOT_CHANGED"));
    assert_eq!(fixture::files(root.path()), files);
}
#[tokio::test]
async fn inline_checksum_is_checked_before_worker() {
    let root = tempfile::tempdir().unwrap();
    let state = fixture::state(root.path()).await;
    let mut project = fixture::project(&state, 4).await;
    project.files[0].sha256 = Some("a".repeat(64));
    fixture::replace(&state, &project, false).await;
    let result = run(
        &state,
        &project.id,
        project.current_revision.as_deref().unwrap(),
        |_, _| async { panic!("must not inspect") },
    )
    .await;
    assert!(matches!(result,Err(AppError::Invalid(ref v)) if v == "PROJECT_SNAPSHOT_CHANGED"));
}

#[tokio::test]
async fn cancellation_after_genuine_worker_completion_rejects_properties_and_keeps_project_unchanged(
) {
    let root = tempfile::tempdir().unwrap();
    let state = fixture::state(root.path()).await;
    let project = saved_step(&state).await;
    let before = fixture::logical(&state, &project.id).await;
    let files = fixture::files(root.path());
    let result = run(
        &state,
        &project.id,
        project.current_revision.as_deref().unwrap(),
        |cwd, cancel| async move {
            let properties =
                crate::native::inspection::inspect_with_executable(&cwd, cancel.clone(), &worker())
                    .await?;
            assert!((properties.volume_mm3 - 8000.).abs() < 1e-6);
            cancel.store(true, Ordering::Relaxed);
            Ok(properties)
        },
    )
    .await;
    assert!(matches!(result, Err(AppError::Invalid(ref code)) if code == "CAD_TASK_CANCELLED"));
    assert_eq!(fixture::logical(&state, &project.id).await, before);
    assert_eq!(fixture::files(root.path()), files);
    // A later request can still acquire the queue after the cancelled host drops its permit.
    let properties = run(
        &state,
        &project.id,
        project.current_revision.as_deref().unwrap(),
        |cwd, cancel| async move {
            crate::native::inspection::inspect_with_executable(&cwd, cancel, &worker()).await
        },
    )
    .await
    .unwrap()
    .unwrap();
    assert!((properties.volume_mm3 - 8000.).abs() < 1e-6);
    assert_eq!(fixture::logical(&state, &project.id).await, before);
    assert_eq!(fixture::files(root.path()), files);
}

#[tokio::test]
async fn close_and_reopen_after_verified_worker_rejects_late_step_properties() {
    let root = tempfile::tempdir().unwrap();
    let state = fixture::state(root.path()).await;
    let project = saved_step(&state).await;
    let source = state.root.join(&project.id);
    std::fs::create_dir(&source).unwrap();
    std::fs::write(source.join("sentinel.txt"), b"unchanged measurement source").unwrap();
    let before = fixture::logical(&state, &project.id).await;
    let files = fixture::files(&source);
    let called = AtomicBool::new(false);
    let result = run(
        &state,
        &project.id,
        project.current_revision.as_deref().unwrap(),
        |cwd, cancel| {
            let state = &state;
            let id = &project.id;
            let called = &called;
            async move {
                let properties =
                    crate::native::inspection::inspect_with_executable(&cwd, cancel, &worker())
                        .await?;
                assert!((properties.volume_mm3 - 8000.).abs() < 1e-6);
                called.store(true, Ordering::Relaxed);
                state.project_access.release(id).unwrap();
                state.project_access.acquire(&state.root, id).unwrap();
                Ok(properties)
            }
        },
    )
    .await;
    assert!(
        called.load(Ordering::Relaxed),
        "The actual inspection worker must finish before reopen"
    );
    assert!(
        matches!(result, Err(AppError::Invalid(ref code)) if code == "PROJECT_SNAPSHOT_CHANGED")
    );
    assert_eq!(fixture::logical(&state, &project.id).await, before);
    assert_eq!(fixture::files(&source), files);
    assert!(state.cad_tasks.status().unwrap().is_empty());
    let fresh = run(
        &state,
        &project.id,
        project.current_revision.as_deref().unwrap(),
        |cwd, cancel| async move {
            crate::native::inspection::inspect_with_executable(&cwd, cancel, &worker()).await
        },
    )
    .await
    .unwrap()
    .unwrap();
    assert!((fresh.volume_mm3 - 8000.).abs() < 1e-6);
    assert_eq!(fixture::logical(&state, &project.id).await, before);
    assert_eq!(fixture::files(&source), files);
    state.project_access.release(&project.id).unwrap();
}
