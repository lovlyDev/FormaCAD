use super::fixture::*;
use forma_core::{
    cad_ir::{Command, Document},
    modeling::{apply::apply_with_executable, commands::prepare},
};
use serde_json::json;

#[tokio::test]
async fn first_datum_amendment_keeps_one_undo_step() {
    use forma_core::{
        cad_ir::Operation,
        modeling::amend_datum,
        project_history::{self, Direction},
    };
    let root = tempfile::tempdir().unwrap();
    let (state, empty) = empty_fixture(root.path()).await;
    let command = add_datums([1., 2., 3.]).pop().unwrap();
    let request = prepare(
        &state,
        empty.id.clone(),
        vec![command],
        "Datum".into(),
        None,
    )
    .await
    .unwrap();
    let first = apply_with_executable(&state, request, executable())
        .await
        .unwrap()
        .project;
    let mut replacement = amend_datum::status(&state, &empty.id)
        .await
        .unwrap()
        .unwrap();
    let old = snapshot(&state, &empty.id).await;
    replacement.operation = Operation::DatumAxis {
        origin_mm: [0.; 3],
        direction: [0., 0., 1.],
        length_mm: 10.,
    };
    assert!(amend_datum::prepare(&state, empty.id.clone(), replacement)
        .await
        .is_err());
    assert_eq!(snapshot(&state, &empty.id).await, old);
    let mut replacement = amend_datum::status(&state, &empty.id)
        .await
        .unwrap()
        .unwrap();
    replacement.operation = Operation::DatumPoint {
        origin_mm: [1., 2., 3.],
    };
    let request = amend_datum::prepare(&state, empty.id.clone(), replacement)
        .await
        .unwrap();
    let adjusted = apply_with_executable(&state, request, executable())
        .await
        .unwrap()
        .project;
    assert!(adjusted.revisions.starts_with(&first.revisions));
    assert!(adjusted.files.is_empty());
    let latest = amend_datum::status(&state, &empty.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        latest.operation,
        Operation::DatumPoint {
            origin_mm: [1., 2., 3.]
        }
    );
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
    clean(&state).await;
}

fn add_datums(direction: [f64; 3]) -> Vec<Command> {
    serde_json::from_value(json!([
        {"command": "add_feature", "feature": {"id": "datum-plane", "name": "Plane",
            "operation": {"type": "datumPlane", "originMm": [2, -4, 7], "normal": [0, 1, 0], "sideMm": 12}}},
        {"command": "add_feature", "feature": {"id": "datum-axis", "name": "Axis",
            "operation": {"type": "datumAxis", "originMm": [2, -4, 7], "direction": direction, "lengthMm": 16}}},
        {"command": "add_feature", "feature": {"id": "datum-point", "name": "Point",
            "operation": {"type": "datumPoint", "originMm": [2, -4, 7]}}}
    ])).unwrap()
}

#[tokio::test]
async fn datum_only_revision_has_no_solid_artifacts_and_can_be_undone() {
    use forma_core::project_history::{self, Direction};
    let root = tempfile::tempdir().unwrap();
    let (state, empty) = empty_fixture(root.path()).await;
    let request = prepare(
        &state,
        empty.id.clone(),
        add_datums([1., 2., 3.]),
        "Datums".into(),
        None,
    )
    .await
    .unwrap();
    let outcome = apply_with_executable(&state, request, executable())
        .await
        .unwrap();
    assert!(outcome.committed);
    assert_eq!(outcome.project.revisions.len(), 1);
    let revision = &outcome.project.revisions[0];
    assert!(revision.preview.is_none() && revision.source.is_none());
    assert!(outcome.project.files.is_empty());
    let document: Document = serde_json::from_str(revision.program.as_deref().unwrap()).unwrap();
    assert_eq!(document.features.len(), 3);
    assert!(document.bodies.is_empty());
    let undone = project_history::apply_history(
        &state,
        &empty.id,
        Some(&revision.id),
        Direction::Undo,
        None,
    )
    .await
    .unwrap();
    assert!(undone.current_revision.is_none());
    clean(&state).await;
}

#[tokio::test]
async fn datum_batch_preserves_solid_geometry_and_rolls_back_on_error() {
    let root = tempfile::tempdir().unwrap();
    let (state, baseline) = fixture(root.path()).await;
    let before = snapshot(&state, &baseline.id).await;
    let request = |direction| {
        prepare(
            &state,
            baseline.id.clone(),
            add_datums(direction),
            "Datums".into(),
            baseline.current_revision.clone().unwrap(),
        )
    };
    assert!(request([0.; 3]).await.is_err());
    assert_eq!(snapshot(&state, &baseline.id).await, before);

    let valid = request([1., 2., 3.]).await.unwrap();
    assert!(
        apply_with_executable(&state, valid, &root.path().join("missing-worker"))
            .await
            .is_err()
    );
    assert_eq!(snapshot(&state, &baseline.id).await, before);
    let outcome = apply_with_executable(&state, request([1., 2., 3.]).await.unwrap(), executable())
        .await
        .unwrap();
    assert!(outcome.committed);
    assert_eq!(
        outcome.project.revisions.len(),
        baseline.revisions.len() + 1
    );
    let saved = outcome.project.revisions.last().unwrap();
    let document: Document = serde_json::from_str(saved.program.as_deref().unwrap()).unwrap();
    assert_eq!(document.features.len(), 5);
    assert_eq!(document.bodies.len(), 1);
    let body = forma_core::native::build_body(&document, "body").unwrap();
    assert!((body.volume_mm3() - 8000.).abs() < 1e-6);
    clean(&state).await;
}
