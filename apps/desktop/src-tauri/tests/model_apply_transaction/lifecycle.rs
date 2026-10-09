use super::fixture::*;
use forma_core::{modeling::apply::apply_with_executable, permissions::Grant};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

#[tokio::test]
async fn queued_cancel_and_head_change_never_spawn_or_publish_stale_result() {
    let root = tempfile::tempdir().unwrap();
    let (state, baseline) = fixture(root.path()).await;
    let before = snapshot(&state, &baseline.id).await;
    let hold = state
        .cad_tasks
        .acquire(&baseline.id, "fixture", token())
        .await
        .unwrap();
    let candidate = request(&baseline, document(50.));
    let task_state = state.clone();
    let missing = root.path().join("must-not-spawn.exe");
    let job =
        tokio::spawn(async move { apply_with_executable(&task_state, candidate, &missing).await });
    queued(&state).await;
    cancel(&state, &baseline.id).await;
    let error = job.await.unwrap().unwrap_err();
    assert!(error.to_string().contains("CANCELLED"));
    drop(hold);
    assert_eq!(snapshot(&state, &baseline.id).await, before);
    clean(&state).await;

    let hold = state
        .cad_tasks
        .acquire(&baseline.id, "fixture", token())
        .await
        .unwrap();
    let candidate = request(&baseline, document(60.));
    let task_state = state.clone();
    let missing = root.path().join("must-not-spawn.exe");
    let job =
        tokio::spawn(async move { apply_with_executable(&task_state, candidate, &missing).await });
    queued(&state).await;
    // A separate valid committed snapshot wins while the captured request waits.
    let mut winner = baseline.clone();
    let mut revision = winner.revisions.last().unwrap().clone();
    let id = uuid::Uuid::new_v4().to_string();
    revision.id = id.clone();
    revision.parent = winner.current_revision.clone();
    let mut value = document(40.);
    value["revisionId"] = id.clone().into();
    revision.program = Some(value.to_string());
    winner.current_revision = Some(id);
    winner.revisions.push(revision);
    let winner = forma_core::projects::persist(&state, winner, vec![], None)
        .await
        .unwrap();
    let winning_snapshot = snapshot(&state, &winner.id).await;
    drop(hold);
    let error = job.await.unwrap().unwrap_err();
    assert!(error.to_string().contains("Project changed"));
    assert_eq!(snapshot(&state, &winner.id).await, winning_snapshot);
    clean(&state).await;
}

#[tokio::test]
async fn cancel_after_real_worker_before_commit_preserves_every_saved_byte() {
    let root = tempfile::tempdir().unwrap();
    let (state, baseline) = fixture(root.path()).await;
    let before = snapshot(&state, &baseline.id).await;
    let hold = state.writes.lock().await;
    let cad_hold = state
        .cad_tasks
        .acquire(&baseline.id, "fixture", token())
        .await
        .unwrap();
    let candidate = request(&baseline, document(50.));
    let task_state = state.clone();
    let id = baseline.id.clone();
    let job =
        tokio::spawn(
            async move { apply_with_executable(&task_state, candidate, executable()).await },
        );
    queued(&state).await;
    drop(cad_hold);
    tokio::time::timeout(Duration::from_secs(10), async {
        // Holding writes prevents commit. A registered task with no CAD permit and
        // an empty owned workspace means the real build already returned bytes.
        loop {
            let registered = state.tasks.lock().await.contains_key(&id);
            let transient = state.root.join(".transient/model-builds");
            let clean_workspace =
                transient.exists() && std::fs::read_dir(&transient).unwrap().count() == 0;
            if registered && state.cad_tasks.status().unwrap().is_empty() && clean_workspace {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    cancel(&state, &baseline.id).await;
    drop(hold);
    assert!(job
        .await
        .unwrap()
        .unwrap_err()
        .to_string()
        .contains("CANCELLED"));
    assert_eq!(snapshot(&state, &baseline.id).await, before);
    clean(&state).await;
    assert!(
        apply_with_executable(&state, request(&baseline, document(50.)), executable())
            .await
            .unwrap()
            .committed
    );
}

#[tokio::test]
async fn actual_apply_permissions_remain_single_use_and_project_scoped() {
    let root = tempfile::tempdir().unwrap();
    let (state, baseline) = fixture(root.path()).await;
    sqlx::query("DELETE FROM settings WHERE key='confirmations'")
        .execute(&state.pool)
        .await
        .unwrap();
    let before = snapshot(&state, &baseline.id).await;
    let missing = root.path().join("must-not-spawn.exe");
    let error = apply_with_executable(&state, request(&baseline, document(50.)), &missing)
        .await
        .unwrap_err();
    assert!(error.to_string().contains("Approval required"));
    for (project_id, expires) in [
        (
            uuid::Uuid::new_v4().to_string(),
            Instant::now() + Duration::from_secs(60),
        ),
        (baseline.id.clone(), Instant::now() - Duration::from_secs(1)),
    ] {
        state.grants.lock().await.insert(
            uuid::Uuid::new_v4().to_string(),
            Grant {
                project_id,
                action: "modify_project".into(),
                approved: true,
                expires,
            },
        );
        assert!(
            apply_with_executable(&state, request(&baseline, document(50.)), &missing)
                .await
                .unwrap_err()
                .to_string()
                .contains("Approval required")
        );
    }
    assert_eq!(snapshot(&state, &baseline.id).await, before);
    let grant_id = uuid::Uuid::new_v4().to_string();
    state.grants.lock().await.insert(
        grant_id.clone(),
        Grant {
            project_id: baseline.id.clone(),
            action: "modify_project".into(),
            approved: true,
            expires: Instant::now() + Duration::from_secs(60),
        },
    );
    let accepted = apply_with_executable(&state, request(&baseline, document(50.)), executable())
        .await
        .unwrap();
    assert!(!state.grants.lock().await.contains_key(&grant_id));
    assert!(
        apply_with_executable(&state, request(&accepted.project, document(60.)), &missing)
            .await
            .unwrap_err()
            .to_string()
            .contains("Approval required")
    );
    clean(&state).await;
}

#[tokio::test]
async fn busy_job_token_is_preserved_and_readonly_host_cannot_apply() {
    let root = tempfile::tempdir().unwrap();
    let (state, baseline) = fixture(root.path()).await;
    let before = snapshot(&state, &baseline.id).await;
    let existing = token();
    state
        .tasks
        .lock()
        .await
        .insert(baseline.id.clone(), existing.clone());
    let error = apply_with_executable(&state, request(&baseline, document(50.)), executable())
        .await
        .unwrap_err();
    assert!(error.to_string().contains("already running"));
    assert!(Arc::ptr_eq(
        state.tasks.lock().await.get(&baseline.id).unwrap(),
        &existing
    ));
    state.tasks.lock().await.remove(&baseline.id);
    assert_eq!(snapshot(&state, &baseline.id).await, before);
    state
        .project_access
        .acquire(&state.root, &baseline.id)
        .unwrap();
    let other = super::fixture::state(root.path()).await;
    let status = other
        .project_access
        .acquire(&other.root, &baseline.id)
        .unwrap();
    assert_eq!(status.mode, "read_only");
    assert!(
        apply_with_executable(&other, request(&baseline, document(50.)), executable())
            .await
            .unwrap_err()
            .to_string()
            .contains("PROJECT_READ_ONLY")
    );
    assert_eq!(snapshot(&state, &baseline.id).await, before);
    state.project_access.release(&baseline.id).unwrap();
    clean(&state).await;
}

#[tokio::test]
async fn final_cas_rejects_built_candidate_after_internal_persistence_winner() {
    let root = tempfile::tempdir().unwrap();
    let (state, baseline) = fixture(root.path()).await;
    let writes = state.writes.lock().await;
    let cad_hold = state
        .cad_tasks
        .acquire(&baseline.id, "fixture", token())
        .await
        .unwrap();
    let candidate = request(&baseline, document(50.));
    let task_state = state.clone();
    let id = baseline.id.clone();
    let job =
        tokio::spawn(
            async move { apply_with_executable(&task_state, candidate, executable()).await },
        );
    queued(&state).await;
    drop(cad_hold);
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let registered = state.tasks.lock().await.contains_key(&id);
            let transient = state.root.join(".transient/model-builds");
            let clean_workspace =
                transient.exists() && std::fs::read_dir(&transient).unwrap().count() == 0;
            if registered && state.cad_tasks.status().unwrap().is_empty() && clean_workspace {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    // This is an internal persistence race fixture under the actual write mutex,
    // not a claim that normal approved UI modeling can bypass the busy-task guard.
    let mut winner = baseline.clone();
    let mut revision = winner.revisions.last().unwrap().clone();
    let id = uuid::Uuid::new_v4().to_string();
    revision.id = id.clone();
    revision.parent = winner.current_revision.clone();
    let mut source = document(40.);
    source["revisionId"] = id.clone().into();
    revision.program = Some(source.to_string());
    winner.current_revision = Some(id.clone());
    winner.revisions.push(revision);
    let winner = forma_core::projects::persist(&state, winner, vec![], None)
        .await
        .unwrap();
    let after_winner = snapshot(&state, &winner.id).await;
    drop(writes);
    let error = job.await.unwrap().unwrap_err();
    assert!(
        error
            .to_string()
            .contains("Project changed during modeling"),
        "{error}"
    );
    assert_eq!(snapshot(&state, &winner.id).await, after_winner);
    let saved = forma_core::projects::get(&state, &winner.id).await.unwrap();
    assert_eq!(saved.current_revision.as_deref(), Some(id.as_str()));
    let saved_source: serde_json::Value =
        serde_json::from_str(saved.revisions.last().unwrap().program.as_ref().unwrap()).unwrap();
    assert_eq!(saved_source["features"][0]["operation"]["width"]["mm"], 40.);
    assert_eq!(saved.revisions.len(), baseline.revisions.len() + 1);
    clean(&state).await;
}
