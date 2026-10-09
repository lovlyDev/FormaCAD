use super::fixture::*;
use forma_core::{
    agents::review_plan::{ExpectedCheck, ReviewPlan},
    modeling::apply::apply_with_executable,
    project_history::{self, Direction},
};
use serde_json::json;

#[tokio::test]
async fn failed_kernel_reference_and_radius_never_commit_artifacts_or_history() {
    let root = tempfile::tempdir().unwrap();
    let (state, baseline) = fixture(root.path()).await;
    let before = snapshot(&state, &baseline.id).await;
    for (candidate, expected) in [
        (
            fillet("other_pad", json!(["turned"]), 1.25),
            "TOPOLOGY_REFERENCE_UNRESOLVED",
        ),
        (
            fillet("pad", json!([]), 1.25),
            "TOPOLOGY_REFERENCE_UNRESOLVED",
        ),
        (fillet("pad", json!(["turned"]), 1000.), "FILLET_FAILED"),
    ] {
        // This must pass IR schema validation, then fail in the real isolated kernel.
        let parsed: forma_core::cad_ir::Document =
            serde_json::from_value(candidate.clone()).unwrap();
        parsed.validate().unwrap();
        let error = apply_with_executable(&state, request(&baseline, candidate), executable())
            .await
            .unwrap_err();
        assert!(error.to_string().contains(expected), "{error}");
        assert_eq!(snapshot(&state, &baseline.id).await, before);
        clean(&state).await;
    }
    let positive = apply_with_executable(
        &state,
        request(&baseline, fillet("pad", json!(["turned"]), 1.25)),
        executable(),
    )
    .await
    .unwrap();
    assert!(positive.committed);
    assert_eq!(
        positive.project.revisions.len(),
        baseline.revisions.len() + 1
    );
    clean(&state).await;
}

#[tokio::test]
async fn actual_apply_seals_one_revision_noop_and_undo_redo_restore_geometry() {
    let root = tempfile::tempdir().unwrap();
    let (state, baseline) = fixture(root.path()).await;
    let before_events: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM project_history_events WHERE project_id=?")
            .bind(&baseline.id)
            .fetch_one(&state.pool)
            .await
            .unwrap();
    let edited = apply_with_executable(&state, request(&baseline, document(50.)), executable())
        .await
        .unwrap();
    assert!(edited.committed);
    let saved = &edited.project.revisions.last().unwrap();
    let doc: serde_json::Value = serde_json::from_str(saved.program.as_ref().unwrap()).unwrap();
    assert_eq!(doc["revisionId"].as_str(), Some(saved.id.as_str()));
    for name in [
        saved.source.as_ref().unwrap(),
        saved.preview.as_ref().unwrap(),
    ] {
        let file = edited
            .project
            .files
            .iter()
            .find(|file| &file.name == name)
            .unwrap();
        let bytes = forma_core::artifacts::read(&state, &edited.project.id, file).unwrap();
        assert_eq!(
            file.sha256.as_deref(),
            Some(forma_core::artifacts::digest(&bytes).as_str())
        );
        assert_eq!(file.size, bytes.len() as u64);
    }
    let after_events: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM project_history_events WHERE project_id=?")
            .bind(&baseline.id)
            .fetch_one(&state.pool)
            .await
            .unwrap();
    assert_eq!(after_events, before_events + 1);
    let before = snapshot(&state, &baseline.id).await;
    sqlx::query("DELETE FROM settings WHERE key='confirmations'")
        .execute(&state.pool)
        .await
        .unwrap();
    let permission_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM permissions WHERE project_id=?")
            .bind(&baseline.id)
            .fetch_one(&state.pool)
            .await
            .unwrap();
    assert!(state.grants.lock().await.is_empty());
    let noop = apply_with_executable(&state, request(&edited.project, doc), executable())
        .await
        .unwrap();
    assert!(!noop.committed);
    assert_eq!(snapshot(&state, &baseline.id).await, before);
    let after_noop_permissions: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM permissions WHERE project_id=?")
            .bind(&baseline.id)
            .fetch_one(&state.pool)
            .await
            .unwrap();
    assert_eq!(after_noop_permissions, permission_count);
    automatic(&state).await;
    let undone = project_history::apply_history(
        &state,
        &baseline.id,
        edited.project.current_revision.as_deref(),
        Direction::Undo,
        None,
    )
    .await
    .unwrap();
    let undo_source: serde_json::Value =
        serde_json::from_str(undone.revisions.last().unwrap().program.as_ref().unwrap()).unwrap();
    assert_eq!(undo_source["features"][0]["operation"]["width"]["mm"], 40.);
    let redone = project_history::apply_history(
        &state,
        &baseline.id,
        undone.current_revision.as_deref(),
        Direction::Redo,
        None,
    )
    .await
    .unwrap();
    let redo_source: serde_json::Value =
        serde_json::from_str(redone.revisions.last().unwrap().program.as_ref().unwrap()).unwrap();
    assert_eq!(redo_source["features"][0]["operation"]["width"]["mm"], 50.);
    clean(&state).await;
}

#[tokio::test]
async fn fresh_review_metrics_can_reject_valid_kernel_output_without_publication() {
    let root = tempfile::tempdir().unwrap();
    let (state, baseline) = fixture(root.path()).await;
    let before = snapshot(&state, &baseline.id).await;
    let mut candidate = request(&baseline, document(50.));
    candidate.review_plan = Some(ReviewPlan {
        assumptions: vec![],
        dimensions: vec![],
        affected_body_ids: vec!["body".into()],
        expected_checks: vec![ExpectedCheck::Volume {
            value_mm3: 1234.,
            tolerance_mm3: 0.01,
        }],
    });
    let error = apply_with_executable(&state, candidate, executable())
        .await
        .unwrap_err();
    assert!(error.to_string().contains("EXPECTED_CHECK_FAILED"));
    assert_eq!(snapshot(&state, &baseline.id).await, before);
    clean(&state).await;
    let mut candidate = request(&baseline, document(50.));
    candidate.review_plan = Some(ReviewPlan {
        assumptions: vec![],
        dimensions: vec![],
        affected_body_ids: vec!["body".into()],
        expected_checks: vec![
            ExpectedCheck::ValidSolid,
            ExpectedCheck::Volume {
                value_mm3: 10000.,
                tolerance_mm3: 0.01,
            },
        ],
    });
    assert!(
        apply_with_executable(&state, candidate, executable())
            .await
            .unwrap()
            .committed
    );
    clean(&state).await;
}

#[tokio::test]
async fn prepared_command_batch_uses_same_host_transaction_and_revision_identity() {
    let root = tempfile::tempdir().unwrap();
    let (state, baseline) = fixture(root.path()).await;
    let commands = vec![serde_json::from_value(
        json!({"command":"set_literal","featureId":"profile","field":"width","valueMm":55.}),
    )
    .unwrap()];
    let request = forma_core::modeling::commands::prepare(
        &state,
        baseline.id.clone(),
        commands,
        "Command edit".into(),
        baseline.current_revision.clone().unwrap(),
    )
    .await
    .unwrap();
    let staged: serde_json::Value = serde_json::from_str(&request.program).unwrap();
    let expected_id = staged["revisionId"].as_str().unwrap().to_owned();
    let accepted = apply_with_executable(&state, request, executable())
        .await
        .unwrap();
    assert!(accepted.committed);
    assert_eq!(
        accepted.project.current_revision.as_deref(),
        Some(expected_id.as_str())
    );
    assert_eq!(
        accepted.project.revisions.last().unwrap().parent,
        baseline.current_revision
    );
    assert_eq!(
        accepted.project.revisions.len(),
        baseline.revisions.len() + 1
    );
    clean(&state).await;
}
