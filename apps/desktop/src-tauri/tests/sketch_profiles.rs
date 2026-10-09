#![cfg(feature = "native-occt")]
use std::{
    io::Write,
    process::{Command, Stdio},
};
fn fixture() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../../docs/fixtures/sketch-holes.cad.json"
    ))
    .unwrap()
}
fn build(document: serde_json::Value, cwd: &std::path::Path) -> serde_json::Value {
    let request = serde_json::json!({"protocolVersion":1,"requestId":"holes_test","bodyId":"body","document":document});
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
fn exact_two_hole_extrusion_in_each_plane_and_direction() {
    for plane in ["xy", "xz", "yz"] {
        for distance in [5.0, -5.0] {
            let mut document = fixture();
            document["features"][0]["operation"]["plane"] = plane.into();
            document["features"][0]["operation"]["originMm"] = serde_json::json!([3, 7, 11]);
            document["features"][1]["operation"]["distance"]["mm"] = distance.into();
            let cwd = tempfile::tempdir().unwrap();
            let result = build(document, cwd.path());
            assert_eq!(result["status"], "completed", "{result}");
            assert!(
                (result["volumeMm3"].as_f64().unwrap() - 5520.0).abs() < 0.001,
                "{result}"
            );
            assert_eq!(result["faceCount"], 14);
            for file in ["model.step", "preview.glb"] {
                assert!(cwd.path().join(file).metadata().unwrap().len() > 100);
            }
        }
    }
}
#[test]
fn profile_crossing_boundary_rejects_before_export() {
    let mut document = fixture();
    document["features"][0]["operation"]["points"][4]["xMm"] = (-2).into();
    let cwd = tempfile::tempdir().unwrap();
    let result = build(document, cwd.path());
    assert_ne!(result["status"], "completed", "{result}");
    assert_eq!(result["code"], "INVALID_SKETCH");
    for file in ["model.step", "preview.glb"] {
        assert!(!cwd.path().join(file).exists());
    }
}
