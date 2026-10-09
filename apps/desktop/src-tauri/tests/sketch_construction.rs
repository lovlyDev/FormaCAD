#![cfg(feature = "native-occt")]
use std::{
    io::Write,
    process::{Command, Stdio},
};

fn request(distance: f64) -> serde_json::Value {
    let mut document: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../docs/fixtures/sketch-construction.cad.json"
    ))
    .unwrap();
    document["parameters"][1]["valueMm"] = serde_json::json!(distance);
    document["features"][0]["operation"]["constraints"].as_array_mut().unwrap().push(
        serde_json::json!({"id":"height","kind":"length","lineId":"bc","distance":{"kind":"literal","mm":10.0}})
    );
    serde_json::json!({"protocolVersion":1,"requestId":"construction_test","bodyId":"body","document":document})
}

fn build(request: serde_json::Value, cwd: &std::path::Path) -> serde_json::Value {
    let mut child = Command::new(env!("CARGO_BIN_EXE_forma-cad-worker"))
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
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
    serde_json::from_slice(&std::fs::read(cwd.join("result.json")).unwrap()).unwrap()
}

#[test]
fn construction_diagonal_is_excluded_from_exact_brep_and_export() {
    let cwd = tempfile::tempdir().unwrap();
    let result = build(request(500.0_f64.sqrt()), cwd.path());
    assert_eq!(result["status"], "completed", "{result}");
    assert!((result["volumeMm3"].as_f64().unwrap() - 1000.0).abs() < 0.001);
    assert_eq!(result["faceCount"], 6);
    assert_eq!(result["edgeCount"], 12);
    assert!(cwd.path().join("model.step").metadata().unwrap().len() > 100);
    assert!(cwd.path().join("preview.glb").metadata().unwrap().len() > 100);
    let mut driven = request(1300.0_f64.sqrt());
    driven["document"]["features"][0]["operation"]["constraints"]
        .as_array_mut()
        .unwrap()
        .retain(|item| item["id"] != "height");
    let cwd = tempfile::tempdir().unwrap();
    let result = build(driven, cwd.path());
    assert_eq!(result["status"], "completed", "{result}");
    assert!((result["volumeMm3"].as_f64().unwrap() - 3000.0).abs() < 0.001);
    assert_eq!(result["faceCount"], 6);
}

#[test]
fn conflicting_construction_dimension_publishes_no_geometry() {
    let cwd = tempfile::tempdir().unwrap();
    let result = build(request(100.0), cwd.path());
    assert_ne!(result["status"], "completed", "{result}");
    assert_eq!(result["code"], "SKETCH_CONSTRAINT_CONFLICT", "{result}");
    assert!(!cwd.path().join("model.step").exists());
    assert!(!cwd.path().join("preview.glb").exists());
}
