use super::fixture::*;
use forma_core::{
    cad_ir::{Command, Document},
    modeling::{apply::apply_with_executable, commands::prepare},
    native::{build_body, Solid},
};
use serde_json::json;
#[tokio::test]
async fn variable_fillet_commits_one_revision_and_rolls_back_impossible_radii() {
    let root = tempfile::tempdir().unwrap();
    let (state, baseline) = fixture(root.path()).await;
    let before = snapshot(&state, &baseline.id).await;
    let commands = |start, end| {
        serde_json::from_value::<Vec<Command>>(json!([
      {"command":"add_feature","feature":{"id":"variable","name":"Variable fillet","operation":{"type":"variableFillet","bodyFeatureId":"pad","reference":{"schemaVersion":1,"kind":"edge","ownerFeatureId":"pad","role":"box-edge:z:xmax:ymax","occurrencePath":[]},"radiusStart":{"kind":"literal","mm":start},"radiusEnd":{"kind":"literal","mm":end}}}},
      {"command":"set_body_source","bodyId":"body","sourceFeatureId":"variable"}
    ])).unwrap()
    };
    let invalid = prepare(
        &state,
        baseline.id.clone(),
        commands(50., 50.),
        "Variable fillet".into(),
        baseline.current_revision.clone(),
    )
    .await
    .unwrap();
    let error = apply_with_executable(&state, invalid, executable())
        .await
        .expect_err("Impossible fillet cannot commit");
    assert!(
        error.to_string().contains("VARIABLE_FILLET_FAILED"),
        "{error}"
    );
    assert_eq!(snapshot(&state, &baseline.id).await, before);
    let valid = prepare(
        &state,
        baseline.id.clone(),
        commands(1., 3.),
        "Variable fillet".into(),
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
    assert!(shape.volume_mm3() > 7980. && shape.volume_mm3() < 7998.);
    assert_eq!(shape.face_count(), 7);
    let file = result
        .project
        .files
        .iter()
        .find(|f| Some(f.name.as_str()) == revision.source.as_deref())
        .unwrap();
    let path = root.path().join("variable.step");
    std::fs::write(
        &path,
        forma_core::artifacts::read(&state, &baseline.id, file).unwrap(),
    )
    .unwrap();
    let restored = Solid::read_step(&path).unwrap();
    assert!((restored.volume_mm3() - shape.volume_mm3()).abs() < 1e-3);
    assert!((restored.surface_area_mm2() - shape.surface_area_mm2()).abs() < 1e-3);
    clean(&state).await;
}
