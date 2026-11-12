use super::fixture::*;
use forma_core::{
    cad_ir::{Command, Document},
    modeling::{apply::apply_with_executable, commands::prepare},
    native::{build_body, Solid},
};
use serde_json::json;

#[tokio::test]
async fn face_thickening_adds_a_body_atomically_and_rolls_back_an_unresolved_face() {
    let root = tempfile::tempdir().unwrap();
    let (state, baseline) = fixture(root.path()).await;
    let before = snapshot(&state, &baseline.id).await;
    let commands = |role: &str| {
        serde_json::from_value::<Vec<Command>>(json!([
            {"command":"add_feature","feature":{"id":"thick","name":"Thickened face",
              "operation":{"type":"thickenFace","bodyFeatureId":"pad",
                "reference":{"schemaVersion":1,"kind":"face","ownerFeatureId":"pad","role":role,"occurrencePath":[]},
                "offset":{"kind":"literal","mm":2}}}},
            {"command":"add_body","body":{"id":"plate","name":"Plate","sourceFeatureId":"thick"}}
        ])).unwrap()
    };
    let request = prepare(
        &state,
        baseline.id.clone(),
        commands("cylinder-face:side"),
        "Thicken".into(),
        baseline.current_revision.clone(),
    )
    .await
    .unwrap();
    let error = apply_with_executable(&state, request, executable())
        .await
        .expect_err("missing face must fail");
    assert!(
        error.to_string().contains("TOPOLOGY_REFERENCE_UNRESOLVED"),
        "{error}"
    );
    assert_eq!(snapshot(&state, &baseline.id).await, before);
    let request = prepare(
        &state,
        baseline.id.clone(),
        commands("box-face:zmax"),
        "Thicken".into(),
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
    assert_eq!(document.bodies.len(), 2);
    assert_eq!(document.bodies[0].source_feature_id, "pad");
    let original = build_body(&document, "body").unwrap();
    let plate = build_body(&document, "plate").unwrap();
    assert!((original.volume_mm3() - 8000.).abs() < 1e-6);
    assert!((plate.volume_mm3() - 1600.).abs() < 1e-6);
    assert!((plate.surface_area_mm2() - 1840.).abs() < 1e-6);
    assert!(document
        .features
        .last()
        .unwrap()
        .operation
        .suppression_source()
        .is_none());
    let file = result
        .project
        .files
        .iter()
        .find(|file| Some(file.name.as_str()) == revision.source.as_deref())
        .unwrap();
    let path = root.path().join("thicken.step");
    std::fs::write(
        &path,
        forma_core::artifacts::read(&state, &baseline.id, file).unwrap(),
    )
    .unwrap();
    let restored = Solid::read_step(&path).unwrap();
    assert!((restored.volume_mm3() - 9600.).abs() < 1e-6);
    assert!((restored.surface_area_mm2() - 4640.).abs() < 1e-6);
    assert_eq!((restored.face_count(), restored.edge_count()), (12, 24));
    clean(&state).await;
}
