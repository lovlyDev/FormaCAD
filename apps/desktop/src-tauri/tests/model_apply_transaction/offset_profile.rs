use super::fixture::*;
use forma_core::{
    cad_ir::{Command, Document},
    modeling::{apply::apply_with_executable, commands::prepare},
    native::{build_body, Solid},
};
use serde_json::json;
#[tokio::test]
async fn offset_profile_is_atomic_keeps_sources_and_rolls_back_collapsed_contours() {
    let root = tempfile::tempdir().unwrap();
    let (state, baseline) = fixture(root.path()).await;
    let before = snapshot(&state, &baseline.id).await;
    let commands = |offset| {
        serde_json::from_value::<Vec<Command>>(json!([
  {"command":"add_feature","feature":{"id":"offset","name":"Offset","operation":{"type":"offsetProfile","sketchId":"profile","offset":{"kind":"literal","mm":offset},"join":"intersection"}}},
  {"command":"add_feature","feature":{"id":"offsetPad","name":"Offset pad","operation":{"type":"extrude","sketchId":"offset","distance":{"kind":"literal","mm":10}}}},
  {"command":"set_body_source","bodyId":"body","sourceFeatureId":"offsetPad"}
 ])).unwrap()
    };
    let invalid = prepare(
        &state,
        baseline.id.clone(),
        commands(-11.),
        "Offset".into(),
        baseline.current_revision.clone(),
    )
    .await
    .unwrap();
    let error = apply_with_executable(&state, invalid, executable())
        .await
        .expect_err("collapsed contour must not commit");
    assert!(
        error
            .to_string()
            .contains("PROFILE_OFFSET_TOPOLOGY_CHANGED"),
        "{error}"
    );
    assert_eq!(snapshot(&state, &baseline.id).await, before);
    let valid = prepare(
        &state,
        baseline.id.clone(),
        commands(2.),
        "Offset".into(),
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
    assert!((shape.volume_mm3() - 10560.).abs() < 1e-4);
    assert!((shape.surface_area_mm2() - 3472.).abs() < 1e-4);
    let file = result
        .project
        .files
        .iter()
        .find(|file| Some(file.name.as_str()) == revision.source.as_deref())
        .unwrap();
    let path = root.path().join("offset.step");
    std::fs::write(
        &path,
        forma_core::artifacts::read(&state, &baseline.id, file).unwrap(),
    )
    .unwrap();
    let restored = Solid::read_step(&path).unwrap();
    assert!((restored.volume_mm3() - 10560.).abs() < 1e-4);
    assert!((restored.surface_area_mm2() - 3472.).abs() < 1e-4);
    clean(&state).await;
}
