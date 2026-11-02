#![cfg(feature = "native-occt")]
use forma_core::model_snap::{
    report::verify_report,
    schema::{SnapBinding, SnapQuery},
};
use serde_json::{json, Value};
use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

fn run(directory: &Path, request: &Value) -> Vec<u8> {
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
    fs::read(directory.join("result.json")).unwrap()
}
fn staged() -> (tempfile::TempDir, SnapBinding) {
    let document = json!({"schemaVersion":2,"revisionId":uuid::Uuid::new_v4().to_string(),"parameters":[],"features":[
        {"id":"profile","name":"Profile","operation":{"type":"rectangle","width":{"kind":"literal","mm":20},"depth":{"kind":"literal","mm":10}}},
        {"id":"pad","name":"Pad","operation":{"type":"extrude","sketchId":"profile","distance":{"kind":"literal","mm":5}}},
        {"id":"move","name":"Move","operation":{"type":"transform","bodyFeatureId":"pad","matrix":[1,0,0,10,0,1,0,0,0,0,1,0]}}
    ],"bodies":[{"id":"body","name":"Body","sourceFeatureId":"move"}]});
    let built = tempfile::tempdir().unwrap();
    let response: Value = serde_json::from_slice(&run(
        built.path(),
        &json!({"protocolVersion":1,"requestId":"build-snap","bodyId":"body","document":document}),
    ))
    .unwrap();
    assert_eq!(response["status"], "completed", "{response}");
    let directory = tempfile::tempdir().unwrap();
    let bytes = serde_json::to_vec(&document).unwrap();
    let source = fs::read(built.path().join("model.step")).unwrap();
    fs::write(directory.path().join("document.json"), &bytes).unwrap();
    fs::write(directory.path().join("source.step"), &source).unwrap();
    let binding = SnapBinding {
        revision_id: document["revisionId"].as_str().unwrap().into(),
        body_id: "body".into(),
        source_sha256: forma_core::artifacts::digest(&source),
        source_size: source.len() as u64,
        document_sha256: forma_core::artifacts::digest(&bytes),
        imported_asset_seals: vec![],
    };
    (directory, binding)
}
fn reference(kind: &str, role: &str) -> Value {
    json!({"schemaVersion":1,"ownerFeatureId":"pad","occurrencePath":["move"],"kind":kind,"role":role})
}
fn request(binding: &SnapBinding, query: &Value) -> Value {
    json!({"protocolVersion":1,"operation":"snap_query","requestId":uuid::Uuid::new_v4().to_string(),"bodyId":binding.body_id,"sourceSha256":binding.source_sha256,"sourceSize":binding.source_size,"documentSha256":binding.document_sha256,"snapQuery":query})
}
#[test]
fn isolated_snap_rebuilds_exact_body_and_writes_only_a_sealed_query_report() {
    let (directory, binding) = staged();
    let raw = fs::read(directory.path().join("document.json")).unwrap();
    let source = fs::read(directory.path().join("source.step")).unwrap();
    for (query, expected) in [
        (
            json!({"kind":"edgeMidpoint","reference":reference("edge","box-edge:x:ymin:zmax")}),
            [10., -5., 5.],
        ),
        (
            json!({"kind":"face","reference":reference("face","box-face:zmax"),"rayOriginMm":[12,3,50],"rayDirection":[0,0,-1]}),
            [12., 3., 5.],
        ),
    ] {
        let request = request(&binding, &query);
        let response = run(directory.path(), &request);
        let report = fs::read(directory.path().join("snap.json")).unwrap();
        let query: SnapQuery = serde_json::from_value(query).unwrap();
        let verified = verify_report(
            &response,
            &report,
            request["requestId"].as_str().unwrap(),
            &binding,
            &query,
        )
        .unwrap();
        for (actual, expected) in verified.result.point_mm.iter().zip(expected) {
            assert!((actual - expected).abs() < 1e-7);
        }
        assert_eq!(
            fs::read(directory.path().join("document.json")).unwrap(),
            raw
        );
        assert_eq!(
            fs::read(directory.path().join("source.step")).unwrap(),
            source
        );
        assert!(!directory.path().join("model.step").exists());
        assert!(!directory.path().join("preview.glb").exists());
    }
}
#[test]
fn invalid_seals_wrong_occurrences_and_mixed_operations_cannot_publish_a_snap() {
    let (directory, binding) = staged();
    let query = json!({"kind":"edgeMidpoint","reference":reference("edge","box-edge:x:ymin:zmax")});
    let valid = request(&binding, &query);
    let response = run(directory.path(), &valid);
    let report = fs::read(directory.path().join("snap.json")).unwrap();
    let typed_query: SnapQuery = serde_json::from_value(query.clone()).unwrap();
    verify_report(
        &response,
        &report,
        valid["requestId"].as_str().unwrap(),
        &binding,
        &typed_query,
    )
    .unwrap();
    let mut wrong = request(&binding, &query);
    wrong["snapQuery"]["reference"]["occurrencePath"] = json!(["other"]);
    let response = run(directory.path(), &wrong);
    assert_eq!(
        serde_json::from_slice::<Value>(&response).unwrap()["status"],
        "error"
    );
    assert!(verify_report(
        &response,
        &report,
        wrong["requestId"].as_str().unwrap(),
        &binding,
        &typed_query
    )
    .is_err());
    let mut mixed = request(&binding, &query);
    mixed["measurementQuery"] = json!({"kind":"bodyMetrics"});
    let response: Value = serde_json::from_slice(&run(directory.path(), &mixed)).unwrap();
    assert_eq!(response["code"], "INVALID_REQUEST");
    let mut other_operation = request(&binding, &query);
    other_operation["operation"] = json!("measure_reference");
    let response: Value = serde_json::from_slice(&run(directory.path(), &other_operation)).unwrap();
    assert_eq!(response["code"], "INVALID_REQUEST");
    fs::write(
        directory.path().join("source.step"),
        b"changed staged source",
    )
    .unwrap();
    let response: Value =
        serde_json::from_slice(&run(directory.path(), &request(&binding, &query))).unwrap();
    assert_eq!(response["code"], "ASSET_CHECKSUM_MISMATCH");
    assert_eq!(
        fs::read(directory.path().join("snap.json")).unwrap(),
        report
    );
    assert!(!directory.path().join("model.step").exists());
    assert!(!directory.path().join("preview.glb").exists());
}
