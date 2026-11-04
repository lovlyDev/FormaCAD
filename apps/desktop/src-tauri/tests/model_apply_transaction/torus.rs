use super::fixture::*;
use forma_core::{
    cad_ir::{Command, Document},
    modeling::{apply::apply_with_executable, commands::prepare},
};
use serde_json::json;

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
