use super::*;
use crate::project_repository::legacy::read_test_fixture as fixture;
fn program() -> String {
    std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../docs/fixtures/sketch-holes.cad.json"),
    )
    .unwrap()
}
fn worker() -> std::path::PathBuf {
    option_env!("CARGO_BIN_EXE_forma-cad-worker")
        .map(std::path::PathBuf::from)
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
#[tokio::test]
async fn pending_journal_refuses_before_task_registration_or_worker() {
    let root = tempfile::tempdir().unwrap();
    let state = fixture::state(root.path()).await;
    let project = fixture::project(&state, 4).await;
    fixture::pending(&state, &project.id).await;
    let before = fixture::logical(&state, &project.id).await;
    let files = fixture::files(root.path());
    let result = run(
        &state,
        &project.id,
        &program(),
        project.current_revision,
        |_, _| async { panic!("worker must not run") },
    )
    .await;
    assert!(
        matches!(result, Err(AppError::Invalid(ref v)) if v == "PROJECT_SNAPSHOT_RECOVERY_REQUIRED")
    );
    assert!(state.tasks.lock().await.is_empty());
    assert_eq!(fixture::logical(&state, &project.id).await, before);
    assert_eq!(fixture::files(root.path()), files);
}
#[tokio::test]
async fn genuine_preview_is_read_only_and_rejects_worker_return_head_race() {
    let root = tempfile::tempdir().unwrap();
    let state = fixture::state(root.path()).await;
    let project = fixture::project(&state, 4).await;
    let program = program();
    let before = fixture::logical(&state, &project.id).await;
    let files = fixture::files(root.path());
    let packet = run(
        &state,
        &project.id,
        &program,
        project.current_revision.clone(),
        |p, c| {
            let state = &state;
            let program = &program;
            async move {
                super::super::build::build_with_project(state, &p, program, c, &worker()).await
            }
        },
    )
    .await
    .unwrap();
    let n = u32::from_le_bytes(packet[..4].try_into().unwrap()) as usize;
    assert_eq!(&packet[4 + n..8 + n], b"glTF");
    assert_eq!(fixture::logical(&state, &project.id).await, before);
    assert_eq!(fixture::files(root.path()), files);
    let result = run(
        &state,
        &project.id,
        &program,
        project.current_revision.clone(),
        |p, c| {
            let state = &state;
            let program = &program;
            async move {
                let packet =
                    super::super::build::build_with_project(state, &p, program, c, &worker())
                        .await?;
                fixture::next_head(state, &p).await;
                Ok(packet)
            }
        },
    )
    .await;
    assert!(matches!(result, Err(AppError::Invalid(ref v)) if v == "PROJECT_SNAPSHOT_CHANGED"));
    assert!(state.tasks.lock().await.is_empty());
    assert_eq!(fixture::files(root.path()), files);
}
#[tokio::test]
async fn cancelled_or_failed_runner_always_cleans_task_markers() {
    let root = tempfile::tempdir().unwrap();
    let state = fixture::state(root.path()).await;
    let project = fixture::project(&state, 4).await;
    let result = run(
        &state,
        &project.id,
        &program(),
        project.current_revision.clone(),
        |_, c| async move {
            c.store(true, Ordering::Relaxed);
            Ok(vec![1])
        },
    )
    .await;
    assert!(matches!(result, Err(AppError::Invalid(ref v)) if v == "Model preview cancelled"));
    assert!(state.tasks.lock().await.is_empty());
    assert!(run(
        &state,
        &project.id,
        &program(),
        project.current_revision,
        |_, _| async { Err(AppError::Invalid("worker refused".into())) }
    )
    .await
    .is_err());
    assert!(state.tasks.lock().await.is_empty());
}

#[tokio::test]
async fn close_and_reopen_after_verified_worker_rejects_late_preview_packet() {
    let root = tempfile::tempdir().unwrap();
    let state = fixture::state(root.path()).await;
    let project = fixture::project(&state, 4).await;
    let source = state.root.join(&project.id);
    std::fs::create_dir(&source).unwrap();
    std::fs::write(source.join("sentinel.txt"), b"unchanged preview source").unwrap();
    let program = program();
    let before = fixture::logical(&state, &project.id).await;
    let files = fixture::files(&source);
    let called = AtomicBool::new(false);
    let result = run(
        &state,
        &project.id,
        &program,
        project.current_revision.clone(),
        |saved, cancel| {
            let state = &state;
            let program = &program;
            let called = &called;
            async move {
                let packet = super::super::build::build_with_project(
                    state,
                    &saved,
                    program,
                    cancel,
                    &worker(),
                )
                .await?;
                let n = u32::from_le_bytes(packet[..4].try_into().unwrap()) as usize;
                assert_eq!(&packet[4 + n..8 + n], b"glTF");
                called.store(true, Ordering::Relaxed);
                state.project_access.release(&saved.id).unwrap();
                state
                    .project_access
                    .acquire(&state.root, &saved.id)
                    .unwrap();
                Ok(packet)
            }
        },
    )
    .await;
    assert!(
        called.load(Ordering::Relaxed),
        "The actual preview worker must finish before reopen"
    );
    assert!(
        matches!(result, Err(AppError::Invalid(ref code)) if code == "PROJECT_SNAPSHOT_CHANGED")
    );
    assert!(state.tasks.lock().await.is_empty());
    assert!(state.cad_tasks.status().unwrap().is_empty());
    assert_eq!(fixture::logical(&state, &project.id).await, before);
    assert_eq!(fixture::files(&source), files);
    let fresh = run(
        &state,
        &project.id,
        &program,
        project.current_revision.clone(),
        |saved, cancel| {
            let state = &state;
            let program = &program;
            async move {
                super::super::build::build_with_project(state, &saved, program, cancel, &worker())
                    .await
            }
        },
    )
    .await
    .unwrap();
    let n = u32::from_le_bytes(fresh[..4].try_into().unwrap()) as usize;
    assert_eq!(&fresh[4 + n..8 + n], b"glTF");
    assert!(state.tasks.lock().await.is_empty());
    assert_eq!(fixture::logical(&state, &project.id).await, before);
    assert_eq!(fixture::files(&source), files);
    state.project_access.release(&project.id).unwrap();
}

#[tokio::test]
async fn close_and_reopen_while_final_preview_cleanup_waits_rejects_verified_packet() {
    use std::{future::Future, task::Poll};
    let root = tempfile::tempdir().unwrap();
    let state = fixture::state(root.path()).await;
    let project = fixture::project(&state, 4).await;
    let source = state.root.join(&project.id);
    std::fs::create_dir(&source).unwrap();
    std::fs::write(source.join("sentinel.txt"), b"unchanged preview source").unwrap();
    let snapshot = legacy::capture_project(&state, &project.id).await.unwrap();
    let cancel = Arc::new(AtomicBool::new(false));
    let packet = super::super::build::build_with_project(
        &state,
        &snapshot.project,
        &program(),
        cancel.clone(),
        &worker(),
    )
    .await
    .unwrap();
    let n = u32::from_le_bytes(packet[..4].try_into().unwrap()) as usize;
    assert_eq!(&packet[4 + n..8 + n], b"glTF");
    legacy::recheck(&state, &snapshot).await.unwrap();
    let before = fixture::logical(&state, &project.id).await;
    let files = fixture::files(&source);
    let marker = format!("preview:{}", project.id);
    // This is the production completion function, reached after a genuine
    // checked worker packet. Hold the actual cleanup mutex; no timer or global
    // phase setter is involved. The map entries are ordinary request markers.
    let mut held = state.tasks.lock().await;
    held.insert(project.id.clone(), cancel.clone());
    held.insert(marker.clone(), cancel.clone());
    let completing = complete(&state, &project.id, &marker, &snapshot, &cancel, Ok(packet));
    tokio::pin!(completing);
    std::future::poll_fn(|context| {
        assert!(completing.as_mut().poll(context).is_pending());
        Poll::Ready(())
    })
    .await;
    state.project_access.release(&project.id).unwrap();
    state
        .project_access
        .acquire(&state.root, &project.id)
        .unwrap();
    drop(held);
    let error = completing.await.err().unwrap();
    assert_eq!(error.to_string(), "PROJECT_SNAPSHOT_CHANGED");
    assert!(state.tasks.lock().await.is_empty());
    assert!(state.cad_tasks.status().unwrap().is_empty());
    assert_eq!(fixture::logical(&state, &project.id).await, before);
    assert_eq!(fixture::files(&source), files);
    state.project_access.release(&project.id).unwrap();
}
