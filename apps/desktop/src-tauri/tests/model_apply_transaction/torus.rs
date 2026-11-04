use super::fixture::*;
use forma_core::{
    cad_ir::{Command, Document},
    modeling::{apply::apply_with_executable, commands::prepare},
};
use serde_json::json;

#[tokio::test]
async fn adjusting_first_torus_keeps_one_undo_slot_and_rejects_stale_edits() {
    use forma_core::{
        modeling::amend_torus::{self, LastTorusStatus, TorusRadii, TorusReplacement},
        project_history::{self, Direction},
    };
    let root = tempfile::tempdir().unwrap();
    let (state, empty) = empty_fixture(root.path()).await;
    let request = prepare(
        &state,
        empty.id.clone(),
        add_torus(3.),
        "Torus".into(),
        None,
    )
    .await
    .unwrap();
    let first = apply_with_executable(&state, request, executable())
        .await
        .unwrap()
        .project;
    let replacement = |status: &LastTorusStatus, radius| TorusReplacement {
        expected_revision: status.expected_revision.clone(),
        expected_generation: status.expected_generation,
        transaction_id: status.transaction_id.clone(),
        radii: TorusRadii {
            major_radius_mm: radius,
            minor_radius_mm: 3.,
        },
    };
    let status = amend_torus::status(&state, &empty.id)
        .await
        .unwrap()
        .unwrap();
    let before = snapshot(&state, &empty.id).await;
    assert!(
        amend_torus::prepare(&state, empty.id.clone(), replacement(&status, 3.))
            .await
            .is_err()
    );
    assert_eq!(snapshot(&state, &empty.id).await, before);
    let request = amend_torus::prepare(&state, empty.id.clone(), replacement(&status, 12.))
        .await
        .unwrap();
    let adjusted = apply_with_executable(&state, request, executable())
        .await
        .unwrap()
        .project;
    assert_eq!(adjusted.revisions.len(), 2);
    assert!(adjusted.revisions.starts_with(&first.revisions));
    assert!(
        amend_torus::prepare(&state, empty.id.clone(), replacement(&status, 14.))
            .await
            .is_err()
    );
    let current = amend_torus::status(&state, &empty.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(current.radii.major_radius_mm, 12.);
    let mut connection = state.pool.acquire().await.unwrap();
    let archive = project_history::export_history(&mut connection, &adjusted)
        .await
        .unwrap();
    project_history::validate_archive(&adjusted, &archive).unwrap();
    drop(connection);
    let undone = project_history::apply_history(
        &state,
        &empty.id,
        adjusted.current_revision.as_deref(),
        Direction::Undo,
        None,
    )
    .await
    .unwrap();
    assert!(undone.current_revision.is_none());
    assert!(amend_torus::status(&state, &empty.id)
        .await
        .unwrap()
        .is_none());
    clean(&state).await;
}

fn add_torus(minor_mm: f64) -> Vec<Command> {
    serde_json::from_value(json!([
        {
            "command": "add_feature",
            "feature": {
                "id": "torus",
                "name": "Torus",
                "operation": {
                    "type": "torus",
                    "majorRadius": {"kind": "literal", "mm": 10.0},
                    "minorRadius": {"kind": "literal", "mm": minor_mm}
                }
            }
        },
        {
            "command": "add_body",
            "body": {"id": "ring", "name": "Ring", "sourceFeatureId": "torus"}
        }
    ]))
    .unwrap()
}

#[tokio::test]
async fn torus_command_commits_one_revision_and_failed_attempts_leave_no_artifacts() {
    let root = tempfile::tempdir().unwrap();
    let (state, baseline) = fixture(root.path()).await;
    let before = snapshot(&state, &baseline.id).await;
    let prepare_torus = |minor| {
        prepare(
            &state,
            baseline.id.clone(),
            add_torus(minor),
            "Torus".into(),
            baseline.current_revision.clone().unwrap(),
        )
    };
    assert!(prepare_torus(10.).await.is_err());
    assert_eq!(snapshot(&state, &baseline.id).await, before);

    let valid = prepare_torus(3.).await.unwrap();
    let missing_worker = root.path().join("missing-worker");
    assert!(apply_with_executable(&state, valid, &missing_worker)
        .await
        .is_err());
    assert_eq!(snapshot(&state, &baseline.id).await, before);
    clean(&state).await;

    let outcome = apply_with_executable(&state, prepare_torus(3.).await.unwrap(), executable())
        .await
        .unwrap();
    assert!(outcome.committed);
    assert_eq!(
        outcome.project.revisions.len(),
        baseline.revisions.len() + 1
    );
    let revision = outcome.project.revisions.last().unwrap();
    let document: Document = serde_json::from_str(revision.program.as_deref().unwrap()).unwrap();
    assert!(document.bodies.iter().any(|body| body.id == "ring"));
    let torus = forma_core::native::build_body(&document, "ring").unwrap();
    let expected = 180.0 * std::f64::consts::PI.powi(2);
    assert!((torus.volume_mm3() - expected).abs() < 1e-5);
    clean(&state).await;
}

#[tokio::test]
async fn torus_can_start_a_document_without_overwriting_a_concurrent_first_revision() {
    let root = tempfile::tempdir().unwrap();
    let (state, empty) = empty_fixture(root.path()).await;
    let prepare_first = || {
        prepare(
            &state,
            empty.id.clone(),
            add_torus(3.),
            "Torus".into(),
            None,
        )
    };
    let initial = prepare_first().await.unwrap();
    let competing = prepare_first().await.unwrap();
    let first = apply_with_executable(&state, initial, executable())
        .await
        .unwrap();
    assert!(first.committed);
    assert_eq!(first.project.revisions.len(), 1);
    assert!(first.project.revisions[0].parent.is_none());
    let before = snapshot(&state, &empty.id).await;
    assert!(apply_with_executable(&state, competing, executable())
        .await
        .is_err());
    assert!(prepare_first().await.is_err());
    assert_eq!(snapshot(&state, &empty.id).await, before);
    clean(&state).await;
}
