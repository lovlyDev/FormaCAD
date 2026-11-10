use super::fixture::*;
use forma_core::{
    cad_ir::{Command, Document},
    modeling::{apply::apply_with_executable, commands::prepare},
    native::{build_body, Solid},
};
use serde_json::json;

#[tokio::test]
async fn boolean_cut_keeps_the_tool_body_and_rolls_back_an_empty_intersection() {
    let root = tempfile::tempdir().unwrap();
    let (state, empty) = empty_fixture(root.path()).await;
    let mut pair = document(40.);
    pair["features"].as_array_mut().unwrap().extend([
        json!({"id":"tool_profile","name":"Tool profile","operation":{"type":"rectangle",
            "width":{"kind":"literal","mm":10},"depth":{"kind":"literal","mm":10}}}),
        json!({"id":"tool_pad","name":"Tool pad","operation":{"type":"extrude",
            "sketchId":"tool_profile","distance":{"kind":"literal","mm":10}}}),
        json!({"id":"tool_offset","name":"Tool offset","operation":{"type":"translate",
            "bodyFeatureId":"tool_pad","offsetMm":[20,0,0]}}),
    ]);
    pair["bodies"].as_array_mut().unwrap().push(json!({
        "id":"tool","name":"Tool","sourceFeatureId":"tool_offset"
    }));
    let baseline = apply_with_executable(&state, request(&empty, pair), executable())
        .await
        .unwrap()
        .project;
    let before = snapshot(&state, &baseline.id).await;
    let draft = |mode, tool| {
        json!({"command":"add_feature","feature":{"id":"result","name":"Result",
        "operation":{"type":"boolean","mode":mode,"leftFeatureId":"pad","rightFeatureId":tool}}})
    };
    let set_output =
        json!({"command":"set_body_source","bodyId":"body","sourceFeatureId":"result"});
    let prepare_commands = |value| {
        prepare(
            &state,
            baseline.id.clone(),
            serde_json::from_value::<Vec<Command>>(value).unwrap(),
            "Boolean operation".into(),
            baseline.current_revision.clone().unwrap(),
        )
    };
    let disjoint = prepare_commands(json!([
        {"command":"add_feature","feature":{"id":"far","name":"Far",
            "operation":{"type":"translate","bodyFeatureId":"tool_offset","offsetMm":[100,0,0]}}},
        draft("intersect", "far"), set_output.clone()
    ]))
    .await
    .unwrap();
    let error = apply_with_executable(&state, disjoint, executable())
        .await
        .err()
        .expect("empty intersection must fail");
    assert!(error.to_string().contains("EMPTY_INTERSECTION"), "{error}");
    assert_eq!(snapshot(&state, &baseline.id).await, before);

    let request = prepare_commands(json!([draft("cut", "tool_offset"), set_output]))
        .await
        .unwrap();
    let result = apply_with_executable(&state, request, executable())
        .await
        .unwrap();
    assert!(result.committed);
    assert_eq!(result.project.revisions.len(), baseline.revisions.len() + 1);
    let revision = result.project.revisions.last().unwrap();
    let document: Document = serde_json::from_str(revision.program.as_deref().unwrap()).unwrap();
    assert_eq!(document.bodies[0].source_feature_id, "result");
    assert_eq!(document.bodies[1].source_feature_id, "tool_offset");
    let target = build_body(&document, "body").unwrap();
    let tool = build_body(&document, "tool").unwrap();
    assert!((target.volume_mm3() - 7500.).abs() < 1e-6);
    assert!((tool.volume_mm3() - 1000.).abs() < 1e-6);
    let file = result
        .project
        .files
        .iter()
        .find(|file| Some(file.name.as_str()) == revision.source.as_deref())
        .unwrap();
    let path = root.path().join("boolean.step");
    std::fs::write(
        &path,
        forma_core::artifacts::read(&state, &baseline.id, file).unwrap(),
    )
    .unwrap();
    let imported = Solid::read_step(&path).unwrap();
    assert_eq!(
        imported.face_count(),
        target.face_count() + tool.face_count()
    );
    assert_eq!(
        imported.edge_count(),
        target.edge_count() + tool.edge_count()
    );
    assert!((imported.volume_mm3() - 8500.).abs() < 1e-6);
    clean(&state).await;
}
