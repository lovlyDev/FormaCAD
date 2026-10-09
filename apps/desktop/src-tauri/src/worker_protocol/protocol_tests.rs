use super::*;
use serde_json::{json, Value};

fn run(value: Value) -> (tempfile::TempDir, Value) {
    let temporary = tempfile::tempdir().unwrap();
    let bytes = serde_json::to_vec(&value).unwrap();
    super::run(bytes.as_slice(), temporary.path()).unwrap();
    let response =
        serde_json::from_slice(&fs::read(temporary.path().join("result.json")).unwrap()).unwrap();
    (temporary, response)
}

#[test]
fn measurement_fields_never_leak_into_legacy_operations() {
    for operation in ["import_step", "inspect_step", "section_step"] {
        let (directory, response) = run(
            json!({"protocolVersion":1,"requestId":"legacy","operation":operation,
            "documentSha256":"a".repeat(64),"sourceSize":10,"measurementQuery":{"kind":"bodyMetrics"}}),
        );
        assert_eq!(response["code"], "INVALID_REQUEST");
        assert!(!directory.path().join("model.step").exists());
        assert!(!directory.path().join("measurement.json").exists());
    }
}

#[test]
fn measurement_requires_complete_binding_and_rejects_inline_documents() {
    let (_, response) = run(
        json!({"protocolVersion":1,"requestId":uuid::Uuid::new_v4().to_string(),
        "operation":"measure_reference","bodyId":"body","measurementQuery":{"kind":"bodyMetrics"}}),
    );
    assert_eq!(response["code"], "INVALID_REQUEST");
    let (directory, response) = run(
        json!({"protocolVersion":1,"requestId":uuid::Uuid::new_v4().to_string(),
        "operation":"measure_reference","bodyId":"body","measurementQuery":{"kind":"bodyMetrics","ordinal":1}}),
    );
    assert_eq!(response["code"], "INVALID_REQUEST");
    assert!(!directory.path().join("measurement.json").exists());
    let (_, response) = run(
        json!({"protocolVersion":1,"requestId":uuid::Uuid::new_v4().to_string(),
        "operation":"measure_reference","document":{"schemaVersion":2,"revisionId":"candidate","features":[],"bodies":[]}}),
    );
    assert_eq!(response["code"], "INVALID_REQUEST");
}

#[test]
fn fixed_document_checksum_failure_happens_before_any_geometry_or_report_output() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("document.json"), b"{}").unwrap();
    fs::write(
        directory.path().join("source.step"),
        b"sealed fixture source",
    )
    .unwrap();
    let value = json!({"protocolVersion":1,"requestId":uuid::Uuid::new_v4().to_string(),"operation":"measure_reference",
        "bodyId":"body","documentSha256":"a".repeat(64),"sourceSha256":"b".repeat(64),
        "sourceSize":21,"measurementQuery":{"kind":"bodyMetrics"}});
    let bytes = serde_json::to_vec(&value).unwrap();
    super::run(bytes.as_slice(), directory.path()).unwrap();
    let result: Value =
        serde_json::from_slice(&fs::read(directory.path().join("result.json")).unwrap()).unwrap();
    assert_eq!(result["code"], "ASSET_CHECKSUM_MISMATCH");
    for name in [
        "model.step",
        "preview.glb",
        "measurement.json",
        "measurement.json.tmp",
    ] {
        assert!(!directory.path().join(name).exists());
    }
}
