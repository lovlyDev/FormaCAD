use super::fixture::*;
use forma_core::{
    cad_ir::{Command, Document},
    modeling::{apply::apply_with_executable, commands::prepare},
};
use serde_json::json;

#[tokio::test]
async fn referenced_chamfer_commits_one_edge_and_rolls_back_invalid_distance() {
    let root = tempfile::tempdir().unwrap();
    let (state, baseline) = fixture(root.path()).await;
    let before = snapshot(&state, &baseline.id).await;
    let request = |distance| {
        let commands: Vec<Command> = serde_json::from_value(json!([
            {"command":"add_feature","feature":{"id":"bevel","name":"Chamfer",
                "operation":{"type":"chamferReferencedEdge","bodyFeatureId":"pad",
                "distance":{"kind":"literal","mm":distance},
                "reference":{"schemaVersion":1,"kind":"edge","ownerFeatureId":"pad",
                    "role":"box-edge:x:ymin:zmax","occurrencePath":[]}}}},
            {"command":"set_body_source","bodyId":"body","sourceFeatureId":"bevel"}
        ]))
        .unwrap();
        prepare(
            &state,
            baseline.id.clone(),
            commands,
            "Chamfer".into(),
            baseline.current_revision.clone().unwrap(),
        )
    };
    let invalid = request(100.).await.unwrap();
    assert!(apply_with_executable(&state, invalid, executable())
        .await
        .is_err());
    assert_eq!(snapshot(&state, &baseline.id).await, before);

    let next = apply_with_executable(&state, request(2.).await.unwrap(), executable())
        .await
        .unwrap();
    assert!(next.committed);
    assert_eq!(next.project.revisions.len(), baseline.revisions.len() + 1);
    let program = next
        .project
        .revisions
        .last()
        .unwrap()
        .program
        .as_deref()
        .unwrap();
    let document: Document = serde_json::from_str(program).unwrap();
    let body = forma_core::native::build_body(&document, "body").unwrap();
    assert!((body.volume_mm3() - (8000. - 40. * 2.)).abs() < 1e-6);
    clean(&state).await;
}

#[tokio::test]
async fn circular_edge_pair_is_committed_atomically_and_has_exact_volume() {
    let root = tempfile::tempdir().unwrap();
    let (state, empty) = empty_fixture(root.path()).await;
    let mut cylinder = document(40.);
    cylinder["features"][0]["operation"] =
        json!({"type":"circle","radius":{"kind":"literal","mm":10}});
    cylinder["features"][1]["operation"]["distance"]["mm"] = json!(20);
    let baseline = apply_with_executable(&state, request(&empty, cylinder), executable())
        .await
        .unwrap()
        .project;
    let before = snapshot(&state, &baseline.id).await;
    let edge = |role, path| {
        json!({"schemaVersion":1,"kind":"edge","ownerFeatureId":"pad",
        "role":role,"occurrencePath":path})
    };
    let commands = |size, second| {
        serde_json::from_value::<Vec<Command>>(json!([
            {"command":"add_feature","feature":{"id":"bevel","name":"Chamfer",
                "operation":{"type":"chamferReferencedEdge","bodyFeatureId":"pad",
                "distance":{"kind":"literal","mm":size},
                "reference":edge("cylinder-edge:top", json!([])), "additionalReferences":[second]}}},
            {"command":"set_body_source","bodyId":"body","sourceFeatureId":"bevel"}
        ])).unwrap()
    };
    let prepare_chamfer = |commands| {
        prepare(
            &state,
            baseline.id.clone(),
            commands,
            "Chamfer".into(),
            baseline.current_revision.clone().unwrap(),
        )
    };
    assert!(
        prepare_chamfer(commands(2., edge("cylinder-edge:top", json!([]))))
            .await
            .is_err()
    );
    let valid_pair = edge("cylinder-edge:bottom", json!([]));
    let oversized = prepare_chamfer(commands(100., valid_pair.clone()))
        .await
        .unwrap();
    assert!(apply_with_executable(&state, oversized, executable())
        .await
        .is_err());
    assert_eq!(snapshot(&state, &baseline.id).await, before);

    let committed = prepare_chamfer(commands(2., valid_pair)).await.unwrap();
    let output = apply_with_executable(&state, committed, executable())
        .await
        .unwrap();
    assert!(output.committed);
    let document: Document = serde_json::from_str(
        output
            .project
            .revisions
            .last()
            .unwrap()
            .program
            .as_deref()
            .unwrap(),
    )
    .unwrap();
    let body = forma_core::native::build_body(&document, "body").unwrap();
    let expected = std::f64::consts::PI * (2000. - 2. * (40. - 8. / 3.));
    assert!((body.volume_mm3() - expected).abs() < 1e-5);
    clean(&state).await;
}
