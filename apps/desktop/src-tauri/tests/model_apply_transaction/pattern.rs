use super::fixture::*;
use forma_core::{
    cad_ir::{Command, Document},
    modeling::{apply::apply_with_executable, commands::prepare},
};
use serde_json::{json, Value};

fn linear(target: &str, holes: bool, x: f64) -> Value {
    json!({"type":"patternLinear", "bodyFeatureId":target, "repeatHole":holes,
      "layout":{"countU":3,"stepUMm":[x,0.,0.],"countV":1,"stepVMm":[0.,0.,0.]}})
}

fn commands(operation: Value, with_hole: bool) -> Vec<Command> {
    let mut batch = Vec::new();
    if with_hole {
        batch.push(
            json!({"command":"add_feature","feature":{"id":"hole","name":"Hole",
            "operation":{"type":"hole","bodyFeatureId":"pad","centerMm":[-10.,0.],"startZMm":0.,
                "radius":{"kind":"literal","mm":1.},"depth":{"kind":"literal","mm":10.}}}}),
        );
    }
    batch.extend([
        json!({"command":"add_feature","feature":{"id":"repeat","name":"Pattern","operation":operation}}),
        json!({"command":"set_body_source","bodyId":"body","sourceFeatureId":"repeat"}),
    ]);
    serde_json::from_value(Value::Array(batch)).unwrap()
}

#[tokio::test]
async fn patterns_commit_once_and_failed_holes_leave_history_intact() {
    let cases = [
        (linear("pad", false, 45.), false, 24_000.),
        (
            json!({"type":"patternCircular","bodyFeatureId":"pad","repeatHole":false,
            "layout":{"count":4,"axisOriginMm":[50.,0.,0.],"axisDirection":[0.,0.,1.],"stepDeg":90.}}),
            false,
            32_000.,
        ),
        (
            linear("hole", true, 10.),
            true,
            8000. - 30. * std::f64::consts::PI,
        ),
    ];
    for (operation, with_hole, expected) in cases {
        let root = tempfile::tempdir().unwrap();
        let (state, baseline) = fixture(root.path()).await;
        let before = snapshot(&state, &baseline.id).await;
        let request = |batch| {
            prepare(
                &state,
                baseline.id.clone(),
                batch,
                "Pattern".into(),
                baseline.current_revision.clone().unwrap(),
            )
        };
        assert!(request(commands(linear("pad", false, 0.), false))
            .await
            .is_err());
        let failed = request(commands(linear("hole", true, 1000.), true))
            .await
            .unwrap();
        assert!(apply_with_executable(&state, failed, executable())
            .await
            .is_err());
        assert_eq!(snapshot(&state, &baseline.id).await, before);

        let next = apply_with_executable(
            &state,
            request(commands(operation, with_hole)).await.unwrap(),
            executable(),
        )
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
        assert!((body.volume_mm3() - expected).abs() < 1e-6);
        clean(&state).await;
    }
}
