use super::fixture::*;
use forma_core::{
    cad_ir::{Command, Document},
    modeling::{apply::apply_with_executable, commands::prepare},
};
use serde_json::json;

fn commands(distance: f64) -> Vec<Command> {
    serde_json::from_value(json!([
        {"command":"add_feature","feature":{"id":"face-extrusion","name":"Extrude face",
            "operation":{"type":"extrudeFace","bodyFeatureId":"pad","distance":{"kind":"literal","mm":distance},
              "reference":{"schemaVersion":1,"kind":"face","ownerFeatureId":"pad","role":"box-face:zmax","occurrencePath":[]}}}},
        {"command":"set_body_source","bodyId":"body","sourceFeatureId":"face-extrusion"}
    ])).unwrap()
}

#[tokio::test]
async fn face_extrusion_is_atomic_for_add_cut_and_invalid_geometry() {
    for distance in [3., -2.] {
        let root = tempfile::tempdir().unwrap();
        let (state, baseline) = fixture(root.path()).await;
        let before = snapshot(&state, &baseline.id).await;
        let request = |distance| {
            prepare(
                &state,
                baseline.id.clone(),
                commands(distance),
                "Extrude face".into(),
                baseline.current_revision.clone().unwrap(),
            )
        };
        assert!(request(0.).await.is_err());
        let invalid = request(-100.).await.unwrap();
        assert!(apply_with_executable(&state, invalid, executable())
            .await
            .is_err());
        assert_eq!(snapshot(&state, &baseline.id).await, before);
        let next = apply_with_executable(&state, request(distance).await.unwrap(), executable())
            .await
            .unwrap();
        assert!(next.committed);
        assert_eq!(next.project.revisions.len(), baseline.revisions.len() + 1);
        let document: Document = serde_json::from_str(
            next.project
                .revisions
                .last()
                .unwrap()
                .program
                .as_deref()
                .unwrap(),
        )
        .unwrap();
        let body = forma_core::native::build_body(&document, "body").unwrap();
        assert!((body.volume_mm3() - (8000. + 800. * distance)).abs() < 1e-6);
        clean(&state).await;
    }
}
