#![cfg(feature = "native-occt")]

use forma_core::cad_ir::{apply_commands, Command as CadCommand, Document};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

fn run(directory: &Path, request: Value) -> Value {
    let mut command = Command::new(env!("CARGO_BIN_EXE_forma-cad-worker"));
    command
        .current_dir(directory)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let mut child = command.spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(request.to_string().as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&fs::read(directory.join("result.json")).unwrap()).unwrap()
}

fn document() -> Document {
    serde_json::from_value(json!({
        "schemaVersion": 2, "revisionId": uuid::Uuid::new_v4().to_string(), "parameters": [],
        "features": [
            {"id":"profile","name":"Profile","operation":{"type":"rectangle","width":{"kind":"literal","mm":20.},"depth":{"kind":"literal","mm":10.}}},
            {"id":"pad","name":"Pad","operation":{"type":"extrude","sketchId":"profile","distance":{"kind":"literal","mm":5.}}}
        ], "bodies": [{"id":"body","name":"Body","sourceFeatureId":"pad"}]
    })).unwrap()
}

fn measured_plane(document: &Document, occurrence: Vec<&str>) -> [f64; 3] {
    let built = tempfile::tempdir().unwrap();
    let response = run(
        built.path(),
        json!({"protocolVersion":1,"requestId":"build-transform","bodyId":"body","document":document}),
    );
    assert_eq!(response["status"], "completed", "{response}");
    assert!((response["volumeMm3"].as_f64().unwrap() - 1000.).abs() < 1e-5);
    let source = fs::read(built.path().join("model.step")).unwrap();
    let staged = tempfile::tempdir().unwrap();
    let bytes = serde_json::to_vec(document).unwrap();
    fs::write(staged.path().join("document.json"), &bytes).unwrap();
    fs::write(staged.path().join("source.step"), &source).unwrap();
    let response = run(
        staged.path(),
        json!({
            "protocolVersion":1,"operation":"measure_reference","requestId":uuid::Uuid::new_v4().to_string(),"bodyId":"body",
            "documentSha256":format!("{:x}",Sha256::digest(&bytes)),"sourceSha256":format!("{:x}",Sha256::digest(&source)),"sourceSize":source.len(),
            "measurementQuery":{"kind":"planarFace","reference":{"schemaVersion":1,"kind":"face","ownerFeatureId":"pad","role":"box-face:xmin","occurrencePath":occurrence}}
        }),
    );
    assert_eq!(response["status"], "completed", "{response}");
    let report: Value =
        serde_json::from_slice(&fs::read(staged.path().join("measurement.json")).unwrap()).unwrap();
    assert_eq!(report["evaluationSource"], "rebuiltAuthoredBody");
    assert_eq!(report["revisionId"], document.revision_id);
    let point = report["result"]["originMm"].as_array().unwrap();
    [
        point[0].as_f64().unwrap(),
        point[1].as_f64().unwrap(),
        point[2].as_f64().unwrap(),
    ]
}

#[test]
fn transform_body_moves_exact_rebuilt_face_ten_millimetres() {
    let before = document();
    let source = serde_json::to_string(&before).unwrap();
    let after = apply_commands(
        &before,
        &before.revision_id,
        &uuid::Uuid::new_v4().to_string(),
        &[CadCommand::TransformBody {
            body_id: "body".into(),
            feature_id: "move".into(),
            name: "Move".into(),
            matrix: [1., 0., 0., 10., 0., 1., 0., 0., 0., 0., 1., 0.],
        }],
    )
    .unwrap();
    let baseline = measured_plane(&before, vec![]);
    let moved = measured_plane(&after, vec!["move"]);
    for axis in 0..3 {
        assert!((moved[axis] - baseline[axis] - if axis == 0 { 10. } else { 0. }).abs() < 1e-7);
    }
    assert_eq!(serde_json::to_string(&before).unwrap(), source);
}

#[test]
fn malformed_transform_emits_no_geometry_artifacts() {
    let mut invalid = serde_json::to_value(document()).unwrap();
    invalid["features"].as_array_mut().unwrap().push(json!({"id":"bad","name":"Invalid scale","operation":{"type":"transform","bodyFeatureId":"pad","matrix":[0.,0.,0.,0., 0.,1.,0.,0., 0.,0.,1.,0.]}}));
    invalid["bodies"][0]["sourceFeatureId"] = json!("bad");
    let directory = tempfile::tempdir().unwrap();
    let response = run(
        directory.path(),
        json!({"protocolVersion":1,"requestId":"invalid-transform","bodyId":"body","document":invalid}),
    );
    assert_eq!(response["status"], "error");
    assert!(!directory.path().join("model.step").exists());
    assert!(!directory.path().join("preview.glb").exists());
}
