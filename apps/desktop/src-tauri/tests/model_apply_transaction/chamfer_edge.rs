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
