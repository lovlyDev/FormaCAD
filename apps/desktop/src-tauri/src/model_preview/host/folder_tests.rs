use super::*;
use crate::project_repository::sqlite_folder::api::{FolderProject, Publication};
use std::{future::Future, path::Path, task::Poll};
#[allow(dead_code)]
mod fixture {
    use crate as forma_core;
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/reference_measurements_host/fixture.rs"
    ));
}

async fn independent(
    state: &AppState,
    project: &crate::models::Project,
    root: &Path,
) -> (Arc<FolderProject>, String) {
    let revision = project
        .revisions
        .iter()
        .find(|r| Some(&r.id) == project.current_revision.as_ref())
        .unwrap();
    let file = project
        .files
        .iter()
        .find(|f| Some(&f.name) == revision.source.as_ref())
        .unwrap();
    let hash = file.sha256.as_deref().unwrap();
    let program=serde_json::json!({"schemaVersion":2,"revisionId":"draft","parameters":[],"features":[
        {"id":"imported","name":"Imported","operation":{"type":"importStep","assetId":format!("step_{hash}"),"sha256":hash}}
    ],"bodies":[{"id":"body","name":"Body","sourceFeatureId":"imported"}]}).to_string();
    let mut pending = Vec::new();
    for file in &project.files {
        pending.push((
            file.sha256.clone().unwrap(),
            crate::artifacts::read(state, &project.id, file).unwrap(),
        ));
    }
    assert!(matches!(
        FolderProject::create(root, project.clone(), pending, None)
            .await
            .unwrap(),
        Publication::Committed(_)
    ));
    let extension = Path::new(&file.name)
        .extension()
        .unwrap()
        .to_str()
        .unwrap()
        .to_ascii_lowercase();
    let managed = state
        .root
        .join(&project.id)
        .join("attachments")
        .join(format!("{hash}.{extension}"));
    let mut wrong = std::fs::read(&managed).unwrap();
    wrong[0] ^= 1;
    std::fs::write(managed, wrong).unwrap();
    (Arc::new(FolderProject::open(root).await.unwrap()), program)
}
fn checked(packet: &[u8], program: &str) {
    let length = u32::from_le_bytes(packet[..4].try_into().unwrap()) as usize;
    assert_eq!(&packet[4 + length..8 + length], b"glTF");
    let metrics: serde_json::Value = serde_json::from_slice(&packet[4..4 + length]).unwrap();
    assert_eq!(
        metrics["sourceSha256"].as_str(),
        Some(crate::artifacts::digest(program.as_bytes()).as_str())
    );
    assert!((metrics["metrics"]["volumeMm3"].as_f64().unwrap() - 8000.).abs() < 1e-6);
    for (actual, expected) in metrics["metrics"]["boundsMm"]
        .as_array()
        .unwrap()
        .iter()
        .zip([40., 20., 10.])
    {
        assert!((actual.as_f64().unwrap() - expected).abs() < 1e-6);
    }
}
async fn clean(state: &AppState) {
    fixture::clean(state).await;
    let previews = state.root.join(".transient/previews");
    if previews.exists() {
        assert_eq!(
            std::fs::read_dir(previews).unwrap().count(),
            0,
            "Preview transient inputs must be removed"
        );
    }
}

#[tokio::test]
async fn original_folder_preview_stages_import_and_refuses_late_worker_close() {
    let app = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let (state, project) = fixture::fixture(app.path()).await;
    let (source, program) = independent(&state, &project, root.path()).await;
    let before = fixture::snapshot(&state, &project.id).await;
    let database = std::fs::read(root.path().join("project.sqlite")).unwrap();
    let read = RepositoryRead::from_folder_view(
        Arc::clone(&source),
        source.capture_metadata().await.unwrap(),
    )
    .await
    .unwrap();
    let scope = read.task_scope_id().to_owned();
    assert_ne!(scope, project.id);
    let packet = run_bound(
        &state,
        read,
        &program,
        project.current_revision.clone(),
        |read, cancel| {
            let state = &state;
            let program = &program;
            let scope = &scope;
            let project_id = &project.id;
            async move {
                {
                    let tasks = state.tasks.lock().await;
                    assert!(tasks.contains_key(scope));
                    assert!(!tasks.contains_key(project_id));
                }
                super::super::build::build_with_read(
                    state,
                    &read,
                    program,
                    cancel,
                    fixture::executable(),
                )
                .await
            }
        },
    )
    .await
    .unwrap();
    checked(&packet, &program);
    let read = RepositoryRead::from_folder_view(
        Arc::clone(&source),
        source.capture_metadata().await.unwrap(),
    )
    .await
    .unwrap();
    let called = AtomicBool::new(false);
    let result = run_bound(
        &state,
        read,
        &program,
        project.current_revision.clone(),
        |read, cancel| {
            let state = &state;
            let program = &program;
            let source = &source;
            let called = &called;
            async move {
                let packet = super::super::build::build_with_read(
                    state,
                    &read,
                    program,
                    cancel,
                    fixture::executable(),
                )
                .await?;
                checked(&packet, program);
                called.store(true, Ordering::Relaxed);
                source.close();
                Ok(packet)
            }
        },
    )
    .await;
    assert!(called.load(Ordering::Relaxed));
    assert!(matches!(result,Err(AppError::Invalid(ref code))if code=="PROJECT_SNAPSHOT_CHANGED"));
    assert_eq!(fixture::snapshot(&state, &project.id).await, before);
    assert_eq!(
        std::fs::read(root.path().join("project.sqlite")).unwrap(),
        database
    );
    clean(&state).await;
}

#[tokio::test]
async fn original_folder_preview_refuses_close_during_final_cleanup_await() {
    let app = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let (state, project) = fixture::fixture(app.path()).await;
    let (source, program) = independent(&state, &project, root.path()).await;
    let read = RepositoryRead::from_folder_view(
        Arc::clone(&source),
        source.capture_metadata().await.unwrap(),
    )
    .await
    .unwrap();
    let scope = read.task_scope_id().to_owned();
    let marker = format!("preview:{scope}");
    let cancel = fixture::token();
    let packet = super::super::build::build_with_read(
        &state,
        &read,
        &program,
        Arc::clone(&cancel),
        fixture::executable(),
    )
    .await
    .unwrap();
    checked(&packet, &program);
    let before = fixture::snapshot(&state, &project.id).await;
    let database = std::fs::read(root.path().join("project.sqlite")).unwrap();
    let mut held = state.tasks.lock().await;
    held.insert(scope.clone(), Arc::clone(&cancel));
    held.insert(marker.clone(), Arc::clone(&cancel));
    let completing = complete(&state, &scope, &marker, &read, &cancel, Ok(packet));
    tokio::pin!(completing);
    std::future::poll_fn(|context| {
        assert!(completing.as_mut().poll(context).is_pending());
        Poll::Ready(())
    })
    .await;
    source.close();
    drop(held);
    assert!(
        matches!(completing.await,Err(AppError::Invalid(ref code))if code=="PROJECT_SNAPSHOT_CHANGED")
    );
    assert_eq!(fixture::snapshot(&state, &project.id).await, before);
    assert_eq!(
        std::fs::read(root.path().join("project.sqlite")).unwrap(),
        database
    );
    clean(&state).await;
}

#[tokio::test]
async fn equal_project_ids_keep_independent_preview_queue_and_cancel_groups() {
    let app = tempfile::tempdir().unwrap();
    let first_root = tempfile::tempdir().unwrap();
    let second_root = tempfile::tempdir().unwrap();
    let (state, project) = fixture::fixture(app.path()).await;
    let (first_source, program) = independent(&state, &project, first_root.path()).await;
    let first_read = RepositoryRead::from_folder_view(
        Arc::clone(&first_source),
        first_source.capture_metadata().await.unwrap(),
    )
    .await
    .unwrap();
    // Copy from the verified first folder: its managed twin is already corrupt.
    let mut pending = Vec::new();
    for file in &first_read.project().files {
        pending.push((
            file.sha256.clone().unwrap(),
            first_read.read_asset(file).await.unwrap(),
        ));
    }
    assert!(matches!(
        FolderProject::create(
            second_root.path(),
            first_read.project().clone(),
            pending,
            None
        )
        .await
        .unwrap(),
        Publication::Committed(_)
    ));
    let second_source = Arc::new(FolderProject::open(second_root.path()).await.unwrap());
    let second_read = RepositoryRead::from_folder_view(
        Arc::clone(&second_source),
        second_source.capture_metadata().await.unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(first_read.project().id, second_read.project().id);
    let first_scope = first_read.task_scope_id().to_owned();
    let second_scope = second_read.task_scope_id().to_owned();
    assert_ne!(first_scope, second_scope);
    let before = fixture::snapshot(&state, &project.id).await;
    let first_sql = std::fs::read(first_root.path().join("project.sqlite")).unwrap();
    let second_sql = std::fs::read(second_root.path().join("project.sqlite")).unwrap();
    let held = state
        .cad_tasks
        .acquire(&project.id, "hold", fixture::token())
        .await
        .unwrap();
    let launch = |read: RepositoryRead<'static>| {
        let job_state = Arc::clone(&state);
        let request = program.clone();
        let expected = project.current_revision.clone();
        tokio::spawn(async move {
            run_bound(&job_state, read, &request, expected, |read, cancel| {
                let state = &job_state;
                let program = &request;
                async move {
                    super::super::build::build_with_read(
                        state,
                        &read,
                        program,
                        cancel,
                        fixture::executable(),
                    )
                    .await
                }
            })
            .await
        })
    };
    let first = launch(first_read);
    let second = launch(second_read);
    tokio::time::timeout(std::time::Duration::from_secs(15), async {
        loop {
            let tasks = state.cad_tasks.status().unwrap();
            if [&first_scope, &second_scope].into_iter().all(|scope| {
                tasks.iter().any(|task| {
                    task.project_id == *scope
                        && task.kind == "model_preview"
                        && task.state == "queued"
                })
            }) {
                break;
            }
            assert!(
                !first.is_finished() && !second.is_finished(),
                "Both same-ID source requests must enter independent queues"
            );
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    {
        let tasks = state.tasks.lock().await;
        let first_cancel = tasks.get(&format!("preview:{first_scope}")).unwrap();
        let second_cancel = tasks.get(&format!("preview:{second_scope}")).unwrap();
        assert!(!Arc::ptr_eq(first_cancel, second_cancel));
        first_cancel.store(true, Ordering::Relaxed);
        assert!(!second_cancel.load(Ordering::Relaxed));
    }
    drop(held);
    let error = first.await.unwrap().unwrap_err();
    assert!(
        error.to_string().to_ascii_lowercase().contains("cancel"),
        "{error}"
    );
    checked(&second.await.unwrap().unwrap(), &program);
    assert_eq!(fixture::snapshot(&state, &project.id).await, before);
    assert_eq!(
        std::fs::read(first_root.path().join("project.sqlite")).unwrap(),
        first_sql
    );
    assert_eq!(
        std::fs::read(second_root.path().join("project.sqlite")).unwrap(),
        second_sql
    );
    clean(&state).await;
}
