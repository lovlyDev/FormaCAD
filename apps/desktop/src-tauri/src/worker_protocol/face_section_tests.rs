use serde_json::{json, Value};
fn query() -> Value {
    json!({"reference":{"schemaVersion":1,"kind":"face","ownerFeatureId":"pad","role":"box-face:zmax","occurrencePath":[]},"offsetMm":-5,"deflectionMm":0.01})
}
fn run(request: Value) -> Value {
    let stage = tempfile::tempdir().unwrap();
    super::run(
        serde_json::to_vec(&request).unwrap().as_slice(),
        stage.path(),
    )
    .unwrap();
    assert!(!stage.path().join("face-section.json").exists());
    assert!(!stage.path().join("model.step").exists());
    serde_json::from_slice(&std::fs::read(stage.path().join("result.json")).unwrap()).unwrap()
}
#[test]
fn face_section_query_cannot_leak_into_other_worker_operations() {
    for operation in [
        "measure_reference",
        "section_step",
        "import_step",
        "inspect_step",
    ] {
        let result = run(
            json!({"protocolVersion":1,"requestId":"legacy","operation":operation,"faceSectionQuery":query()}),
        );
        assert_eq!(result["code"], "INVALID_REQUEST", "{operation}");
    }
    assert_eq!(
        run(json!({"protocolVersion":1,"requestId":"build","faceSectionQuery":query()}))["code"],
        "INVALID_REQUEST"
    );
}
#[test]
fn face_section_request_rejects_renderer_plane_ordinal_and_caller_paths() {
    let base = json!({"protocolVersion":1,"requestId":uuid::Uuid::new_v4().to_string(),"operation":"section_reference","bodyId":"body","documentSha256":"a".repeat(64),"sourceSha256":"b".repeat(64),"sourceSize":100,"faceSectionQuery":query()});
    for field in ["normal", "originMm", "ordinal", "sourcePath"] {
        let mut request = base.clone();
        request["faceSectionQuery"][field] = json!([0, 0, 1]);
        assert_eq!(run(request)["code"], "INVALID_REQUEST", "{field}");
    }
    let mut direct_plane = base.clone();
    direct_plane["sectionPlane"] = json!({"originMm":[0,0,0],"normal":[0,0,1]});
    assert_eq!(run(direct_plane)["code"], "INVALID_REQUEST");
    let mut inline = base.clone();
    inline["document"] =
        json!({"schemaVersion":2,"revisionId":"candidate","features":[],"bodies":[]});
    assert_eq!(run(inline)["code"], "INVALID_REQUEST");
    let mut measurement = base;
    measurement["measurementQuery"] = json!({"kind":"bodyMetrics"});
    assert_eq!(run(measurement)["code"], "INVALID_REQUEST");
}
