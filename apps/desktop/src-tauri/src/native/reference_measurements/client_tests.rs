use super::*;
use crate::model_measurement::schema::{MeasurementEngine, MeasurementValue};
use serde_json::{json, Value};

fn fixture() -> (String, MeasurementBinding, MeasurementQuery, Value) {
    let request_id = uuid::Uuid::new_v4().to_string();
    let binding = MeasurementBinding {
        revision_id: uuid::Uuid::new_v4().to_string(),
        body_id: "body".into(),
        source_sha256: "a".repeat(64),
        source_size: 120,
        document_sha256: "b".repeat(64),
        imported_asset_seals: vec![],
    };
    let query = MeasurementQuery::BodyMetrics {};
    let report = MeasurementReport {
        schema_version: 1,
        request_id: request_id.clone(),
        engine: MeasurementEngine {
            name: "occt".into(),
            protocol_version: 1,
        },
        evaluation_source: "rebuiltAuthoredBody".into(),
        revision_id: binding.revision_id.clone(),
        body_id: binding.body_id.clone(),
        source_sha256: binding.source_sha256.clone(),
        source_size: binding.source_size,
        document_sha256: binding.document_sha256.clone(),
        imported_asset_seals: vec![],
        query: query.clone(),
        result: MeasurementValue::BodyMetrics {
            volume_mm3: 8000.,
            area_mm2: 2800.,
            face_count: 6,
            edge_count: 12,
            extents_mm: [40., 20., 10.],
        },
    };
    (
        request_id,
        binding,
        query,
        serde_json::to_value(report).unwrap(),
    )
}
fn response(request_id: &str, bytes: &[u8]) -> Vec<u8> {
    serde_json::to_vec(
        &json!({"protocolVersion":1,"requestId":request_id,"status":"completed",
        "measurementSha256":crate::artifacts::digest(bytes)}),
    )
    .unwrap()
}

#[test]
fn separately_sealed_report_still_must_match_every_captured_input_and_query() {
    let (id, binding, query, report) = fixture();
    let bytes = serde_json::to_vec(&report).unwrap();
    assert!(verify_report(&response(&id, &bytes), &bytes, &id, &binding, &query).is_ok());
    for (field, value) in [
        ("bodyId", json!("other")),
        ("revisionId", json!(uuid::Uuid::new_v4().to_string())),
        ("documentSha256", json!("c".repeat(64))),
        ("sourceSha256", json!("d".repeat(64))),
        ("sourceSize", json!(121)),
        ("evaluationSource", json!("committedStep")),
        ("query", json!({"kind":"edgeLength"})),
    ] {
        let mut forged = report.clone();
        forged[field] = value;
        let bytes = serde_json::to_vec(&forged).unwrap();
        // Recompute the transport seal so this tests semantic identity, not checksum alone.
        assert!(
            verify_report(&response(&id, &bytes), &bytes, &id, &binding, &query).is_err(),
            "accepted {field}"
        );
    }
}

#[test]
fn transport_checks_request_id_checksum_bounds_and_exclusive_result_branch() {
    let (id, binding, query, report) = fixture();
    let bytes = serde_json::to_vec(&report).unwrap();
    assert!(verify_report(
        &response("wrong-request", &bytes),
        &bytes,
        &id,
        &binding,
        &query
    )
    .is_err());
    let mut changed = bytes.clone();
    changed.push(b' ');
    assert!(verify_report(&response(&id, &bytes), &changed, &id, &binding, &query).is_err());
    let mut contradictory: Value = serde_json::from_slice(&response(&id, &bytes)).unwrap();
    contradictory["stepSha256"] = "c".repeat(64).into();
    assert!(verify_report(
        &serde_json::to_vec(&contradictory).unwrap(),
        &bytes,
        &id,
        &binding,
        &query
    )
    .is_err());
    assert!(verify_report(
        &response(&id, &bytes),
        &vec![b' '; MAX_REPORT_BYTES + 1],
        &id,
        &binding,
        &query
    )
    .is_err());
    let mut invalid = report;
    invalid["result"]["extentsMm"] = json!([40., 20., -10.]);
    let bytes = serde_json::to_vec(&invalid).unwrap();
    assert!(verify_report(&response(&id, &bytes), &bytes, &id, &binding, &query).is_err());
}
