use super::fixture::*;
use forma_core::{
    cad_ir::{Command, Document},
    modeling::{apply::apply_with_executable, commands::prepare},
};
use serde_json::json;

#[tokio::test]
async fn shell_keeps_failed_builds_out_of_history_and_commits_exact_walls() {
    let root = tempfile::tempdir().unwrap();
    let (state, baseline) = fixture(root.path()).await;
    let before = snapshot(&state, &baseline.id).await;
    let request = |offset| {
        let commands: Vec<Command> = serde_json::from_value(json!([
            {"command":"add_feature","feature":{"id":"walls","name":"Shell",
                "operation":{"type":"shell","bodyFeatureId":"pad",
                "offset":{"kind":"literal","mm":offset},
                "openings":[{"schemaVersion":1,"kind":"face","ownerFeatureId":"pad",
                    "role":"box-face:zmax","occurrencePath":[]}]}}},
            {"command":"set_body_source","bodyId":"body","sourceFeatureId":"walls"}
        ]))
        .unwrap();
        prepare(
            &state,
            baseline.id.clone(),
            commands,
            "Shell".into(),
            baseline.current_revision.clone().unwrap(),
        )
    };
    assert!(request(0.).await.is_err());
    let invalid = request(-100.).await.unwrap();
    assert!(apply_with_executable(&state, invalid, executable())
        .await
        .is_err());
    assert_eq!(snapshot(&state, &baseline.id).await, before);

    let next = apply_with_executable(&state, request(-1.).await.unwrap(), executable())
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
    assert!((body.volume_mm3() - 1844.).abs() < 1e-6);
    clean(&state).await;
}
