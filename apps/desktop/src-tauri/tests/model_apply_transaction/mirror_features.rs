use super::fixture::*;
use forma_core::{
    cad_ir::{Command, Document},
    modeling::{apply::apply_with_executable, commands::prepare},
    native::{build_body, Solid},
};
use serde_json::json;
#[tokio::test]
async fn mirror_keeps_source_features_and_rolls_back_a_duplicate_effect() {
    let root = tempfile::tempdir().unwrap();
    let (state, baseline) = fixture(root.path()).await;
    let mut value = document(40.);
    value["features"].as_array_mut().unwrap().extend([
      json!({"id":"circle","name":"Boss profile","operation":{"type":"circle","radius":{"kind":"literal","mm":2}}}),
      json!({"id":"boss","name":"Boss","operation":{"type":"extrude","sketchId":"circle","distance":{"kind":"literal","mm":4}}}),
      json!({"id":"placed","name":"Placed","operation":{"type":"translate","bodyFeatureId":"boss","offsetMm":[10,0,10]}}),
      json!({"id":"withBoss","name":"Add boss","operation":{"type":"boolean","leftFeatureId":"pad","rightFeatureId":"placed","mode":"union"}})
    ]);
    value["bodies"][0]["sourceFeatureId"] = "withBoss".into();
    let baseline = apply_with_executable(&state, request(&baseline, value), executable())
        .await
        .unwrap()
        .project;
    let before = snapshot(&state, &baseline.id).await;
    let commands = |x| {
        serde_json::from_value::<Vec<Command>>(json!([
      {"command":"add_feature","feature":{"id":"mirrored","name":"Mirror features","operation":{"type":"mirrorFeatures","bodyFeatureId":"withBoss","baseFeatureId":"pad","featureIds":["withBoss"],"planeOriginMm":[x,0,0],"planeNormal":[1,0,0]}}},
      {"command":"set_body_source","bodyId":"body","sourceFeatureId":"mirrored"}
    ])).unwrap()
    };
    let invalid = prepare(
        &state,
        baseline.id.clone(),
        commands(10.),
        "Mirror".into(),
        baseline.current_revision.clone(),
    )
    .await
    .unwrap();
    let error = apply_with_executable(&state, invalid, executable())
        .await
        .expect_err("duplicate effect must fail");
    assert!(
        error.to_string().contains("MIRROR_FEATURES_NO_EFFECT"),
        "{error}"
    );
    assert_eq!(snapshot(&state, &baseline.id).await, before);
    let valid = prepare(
        &state,
        baseline.id.clone(),
        commands(0.),
        "Mirror".into(),
        baseline.current_revision.clone(),
    )
    .await
    .unwrap();
    let result = apply_with_executable(&state, valid, executable())
        .await
        .unwrap();
    assert!(result.committed);
    assert_eq!(result.project.revisions.len(), baseline.revisions.len() + 1);
    let revision = result.project.revisions.last().unwrap();
    let doc: Document = serde_json::from_str(revision.program.as_deref().unwrap()).unwrap();
    let original: Document = serde_json::from_str(
        baseline
            .revisions
            .last()
            .unwrap()
            .program
            .as_deref()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(&doc.features[..original.features.len()], &original.features);
    let shape = build_body(&doc, "body").unwrap();
    let pi = std::f64::consts::PI;
    assert!((shape.volume_mm3() - (8000. + 32. * pi)).abs() < 1e-4);
    assert_eq!((shape.face_count(), shape.edge_count()), (10, 18));
    let file = result
        .project
        .files
        .iter()
        .find(|file| Some(file.name.as_str()) == revision.source.as_deref())
        .unwrap();
    let path = root.path().join("mirror.step");
    std::fs::write(
        &path,
        forma_core::artifacts::read(&state, &baseline.id, file).unwrap(),
    )
    .unwrap();
    let restored = Solid::read_step(&path).unwrap();
    assert!((restored.volume_mm3() - (8000. + 32. * pi)).abs() < 1e-4);
    assert!((restored.surface_area_mm2() - (2800. + 32. * pi)).abs() < 1e-4);
    let mut suppressed = doc.clone();
    suppressed.features.last_mut().unwrap().suppressed = true;
    assert!(
        (build_body(&suppressed, "body").unwrap().volume_mm3() - (8000. + 16. * pi)).abs() < 1e-4
    );
    clean(&state).await;
}
