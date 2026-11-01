//! Real isolated managed SQL/ReadSession and queue tests; no geometry claim.
use super::*;
use crate::project_repository::legacy::read_test_fixture as fixture;
const SOURCE: &str = "result = cq.Workplane('XY').box(40,20,10)";

fn request(project: &Project, source: &str) -> ApplyRequest {
    ApplyRequest::new(
        project.id.clone(),
        source.into(),
        "edit".into(),
        project.current_revision.clone(),
    )
}

fn assert_send<T: Send>(_: T) {}

#[tokio::test]
async fn no_op_needs_no_grant_or_task_and_future_is_send() {
    let directory = tempfile::tempdir().unwrap();
    let state = fixture::state(directory.path()).await;
    let mut project = fixture::project(&state, 32).await;
    project.revisions[0].program = Some(SOURCE.into());
    fixture::replace(&state, &project, false).await;
    let host = managed::ManagedHost {
        state: &state,
        project_id: &project.id,
        executable: None,
    };
    assert_send(apply(
        ApplyRuntime::from_state(&state),
        &host,
        request(&project, SOURCE),
    ));
    let before = fixture::logical(&state, &project.id).await;
    let outcome = apply(
        ApplyRuntime::from_state(&state),
        &host,
        request(&project, SOURCE),
    )
    .await
    .unwrap();
    assert!(matches!(outcome, RepositoryApply::NoOp(_)));
    assert!(state.tasks.lock().await.is_empty());
    assert!(state.cad_tasks.status().unwrap().is_empty());
    assert_eq!(before, fixture::logical(&state, &project.id).await);
    let audit: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM permissions")
        .fetch_one(&state.pool)
        .await
        .unwrap();
    assert_eq!(audit, 0);
}

#[tokio::test]
async fn latest_preserves_metadata_but_rejects_same_head_revision_tampering() {
    let directory = tempfile::tempdir().unwrap();
    let state = fixture::state(directory.path()).await;
    let project = fixture::project(&state, 32).await;
    let host = managed::ManagedHost {
        state: &state,
        project_id: &project.id,
        executable: None,
    };
    let (_access, original) = host.capture().await.unwrap();
    let mut renamed = project.clone();
    renamed.name = "Metadata updated while queued".into();
    fixture::replace(&state, &renamed, false).await;
    let latest = host.latest(&original).await.unwrap();
    assert_eq!(latest.project.name, renamed.name);
    renamed.revisions[0].program = Some(SOURCE.into());
    fixture::replace(&state, &renamed, false).await;
    assert!(host.latest(&original).await.is_err());
}

#[tokio::test]
async fn history_seal_rejects_saved_head_aba_even_when_project_returns_to_original() {
    let directory = tempfile::tempdir().unwrap();
    let state = fixture::state(directory.path()).await;
    let project = fixture::project(&state, 32).await;
    let host = managed::ManagedHost {
        state: &state,
        project_id: &project.id,
        executable: None,
    };
    let (_access, original) = host.capture().await.unwrap();
    fixture::next_head(&state, &project).await;
    // Simulate an old head restored while durable history records the intervening
    // transition. Identical head/files/revisions must not revive the captured job.
    fixture::replace(&state, &project, false).await;
    assert!(host.latest(&original).await.is_err());
}

#[tokio::test]
async fn queued_stale_job_never_enters_worker_and_cleans_own_registration() {
    let directory = tempfile::tempdir().unwrap();
    let state = fixture::state(directory.path()).await;
    let project = fixture::project(&state, 32).await;
    let host = managed::ManagedHost {
        state: &state,
        project_id: &project.id,
        executable: None,
    };
    state.grants.lock().await.insert(
        uuid::Uuid::new_v4().to_string(),
        crate::permissions::Grant {
            project_id: project.id.clone(),
            action: "modify_project".into(),
            approved: true,
            expires: std::time::Instant::now() + std::time::Duration::from_secs(60),
        },
    );
    let occupied = state
        .cad_tasks
        .acquire(
            &project.id,
            "test_barrier",
            Arc::new(AtomicBool::new(false)),
        )
        .await
        .unwrap();
    let mutation = async {
        // Bound the wait so a failed registration fails the test instead of hanging.
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                if state.tasks.lock().await.contains_key(&project.id) {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        fixture::next_head(&state, &project).await;
        drop(occupied);
    };
    let (outcome, ()) = tokio::join!(
        apply(
            ApplyRuntime::from_state(&state),
            &host,
            request(&project, SOURCE)
        ),
        mutation
    );
    match outcome {
        Err(AppError::Invalid(message)) => assert_eq!(
            message,
            "Project changed during modeling; retry the request"
        ),
        _ => panic!("queued stale job reached worker or changed error classification"),
    }
    assert!(state.tasks.lock().await.is_empty());
    assert!(state.cad_tasks.status().unwrap().is_empty());
}
