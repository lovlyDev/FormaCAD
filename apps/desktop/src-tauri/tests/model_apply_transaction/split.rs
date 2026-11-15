use super::fixture::*;
use forma_core::{
    cad_ir::{Command, Document},
    modeling::{apply::apply_with_executable, commands::prepare},
    native::{build_body, Solid},
};
use serde_json::json;
#[tokio::test]
async fn plane_split_commits_both_halves_and_rolls_back_a_missed_plane() {
    let root = tempfile::tempdir().unwrap();
    let (state, baseline) = fixture(root.path()).await;
    let before = snapshot(&state, &baseline.id).await;
    let commands = |x| {
        serde_json::from_value::<Vec<Command>>(json!([
        {"command":"add_feature","feature":{"id":"positive","name":"Positive","operation":{"type":"splitPlane","bodyFeatureId":"pad","planeOriginMm":[x,0,0],"planeNormal":[1,0,0],"side":"positive"}}},
        {"command":"add_feature","feature":{"id":"negative","name":"Negative","operation":{"type":"splitPlane","bodyFeatureId":"pad","planeOriginMm":[x,0,0],"planeNormal":[1,0,0],"side":"negative"}}},
        {"command":"set_body_source","bodyId":"body","sourceFeatureId":"positive"},
        {"command":"add_body","body":{"id":"negative_body","name":"Negative part","sourceFeatureId":"negative"}}
    ])).unwrap()
    };
    let missed = prepare(
        &state,
        baseline.id.clone(),
        commands(100.),
        "Split".into(),
        baseline.current_revision.clone(),
    )
    .await
    .unwrap();
    let error = apply_with_executable(&state, missed, executable())
        .await
        .expect_err("missed plane must fail");
    assert!(
        error.to_string().contains("SPLIT_NO_INTERSECTION"),
        "{error}"
    );
    assert_eq!(snapshot(&state, &baseline.id).await, before);
    let request = prepare(
        &state,
        baseline.id.clone(),
        commands(5.),
        "Split".into(),
        baseline.current_revision.clone(),
    )
    .await
    .unwrap();
    let result = apply_with_executable(&state, request, executable())
        .await
        .unwrap();
    assert!(result.committed);
    assert_eq!(result.project.revisions.len(), baseline.revisions.len() + 1);
    let revision = result.project.revisions.last().unwrap();
    let document: Document = serde_json::from_str(revision.program.as_deref().unwrap()).unwrap();
    let positive = build_body(&document, "body").unwrap();
    let negative = build_body(&document, "negative_body").unwrap();
    assert!((positive.volume_mm3() - 3000.).abs() < 1e-6);
    assert!((negative.volume_mm3() - 5000.).abs() < 1e-6);
    let file = result
        .project
        .files
        .iter()
        .find(|file| Some(file.name.as_str()) == revision.source.as_deref())
        .unwrap();
    let path = root.path().join("split.step");
    std::fs::write(
        &path,
        forma_core::artifacts::read(&state, &baseline.id, file).unwrap(),
    )
    .unwrap();
    let restored = Solid::read_step(&path).unwrap();
    assert!((restored.volume_mm3() - 8000.).abs() < 1e-6);
    assert!((restored.surface_area_mm2() - 3200.).abs() < 1e-6);
    assert_eq!((restored.face_count(), restored.edge_count()), (12, 24));
    clean(&state).await;
}
