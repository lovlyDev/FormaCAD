use super::*;
use crate::{
    model_measurement::schema::MeasurementEngine,
    model_pair_measurement::schema::PairMeasurementValue,
};
use serde_json::{json, Value};
fn fixture() -> (String, MeasurementBinding, PairMeasurementQuery, Value) {
    let id = uuid::Uuid::new_v4().to_string();
    let binding = MeasurementBinding {
        revision_id: uuid::Uuid::new_v4().to_string(),
        body_id: "body".into(),
        source_sha256: "a".repeat(64),
        source_size: 100,
        document_sha256: "b".repeat(64),
        imported_asset_seals: vec![],
    };
    let reference = |role: &str| json!({"schemaVersion":1,"kind":"face","ownerFeatureId":"pad","role":role,"occurrencePath":[]});
    let query:PairMeasurementQuery=serde_json::from_value(json!({"kind":"minimumDistance","first":reference("box-face:xmin"),"second":reference("box-face:xmax")})).unwrap();
    let report = PairMeasurementReport {
        schema_version: 1,
        request_id: id.clone(),
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
        result: PairMeasurementValue::MinimumDistance {
            distance_mm: 40.,
            point_a_mm: [-20., 0., 5.],
            point_b_mm: [20., 0., 5.],
        },
    };
    (id, binding, query, serde_json::to_value(report).unwrap())
}
fn response(id: &str, bytes: &[u8]) -> Vec<u8> {
    serde_json::to_vec(&json!({"protocolVersion":1,"requestId":id,"status":"completed","measurementSha256":crate::artifacts::digest(bytes)})).unwrap()
}
#[test]
fn sealed_pair_report_must_echo_both_references_in_captured_order_and_all_input_bindings() {
    let (id, binding, query, report) = fixture();
    let bytes = serde_json::to_vec(&report).unwrap();
    assert!(verify_report(&response(&id, &bytes), &bytes, &id, &binding, &query).is_ok());
    for (field, value) in [
        ("bodyId", json!("other")),
        ("revisionId", json!(uuid::Uuid::new_v4().to_string())),
        ("sourceSha256", json!("c".repeat(64))),
        ("documentSha256", json!("d".repeat(64))),
        ("sourceSize", json!(101)),
        ("evaluationSource", json!("committedStep")),
    ] {
        let mut forged = report.clone();
        forged[field] = value;
        let bytes = serde_json::to_vec(&forged).unwrap();
        assert!(
            verify_report(&response(&id, &bytes), &bytes, &id, &binding, &query).is_err(),
            "{field}"
        );
    }
    let mut swapped = report.clone();
    swapped["query"]["first"] = report["query"]["second"].clone();
    swapped["query"]["second"] = report["query"]["first"].clone();
    let bytes = serde_json::to_vec(&swapped).unwrap();
    assert!(verify_report(&response(&id, &bytes), &bytes, &id, &binding, &query).is_err());
    let mut forged = report;
    forged["result"]["distanceMm"] = 41.into();
    let bytes = serde_json::to_vec(&forged).unwrap();
    assert!(verify_report(&response(&id, &bytes), &bytes, &id, &binding, &query).is_err());
}

#[test]
fn pair_transport_rejects_wrong_identity_hash_limits_and_other_workflow_outputs() {
    let (id, binding, query, report) = fixture();
    let bytes = serde_json::to_vec(&report).unwrap();
    let base: Value = serde_json::from_slice(&response(&id, &bytes)).unwrap();
    for (field, value) in [
        ("protocolVersion", json!(2)),
        ("requestId", json!(uuid::Uuid::new_v4().to_string())),
        ("status", json!("error")),
        ("code", json!("MEASUREMENT_FAILED")),
        ("detail", json!("unexpected error detail")),
        ("measurementSha256", json!("e".repeat(64))),
        ("volumeMm3", json!(0)),
        ("areaMm2", json!(0)),
        ("faceCount", json!(0)),
        ("edgeCount", json!(0)),
        ("boundsMm", json!([0, 0, 0])),
        ("stepSha256", json!("f".repeat(64))),
        ("previewSha256", json!("f".repeat(64))),
        ("sectionSha256", json!("f".repeat(64))),
        ("unknownOutput", json!(true)),
    ] {
        let mut forged = base.clone();
        forged[field] = value;
        let response = serde_json::to_vec(&forged).unwrap();
        assert!(
            verify_report(&response, &bytes, &id, &binding, &query).is_err(),
            "{field}"
        );
    }
    assert!(verify_report(&[], &bytes, &id, &binding, &query).is_err());
    assert!(verify_report(&vec![b' '; 64 * 1024 + 1], &bytes, &id, &binding, &query).is_err());
    let oversized = vec![b' '; MAX_PAIR_REPORT_BYTES + 1];
    assert!(verify_report(
        &response(&id, &oversized),
        &oversized,
        &id,
        &binding,
        &query
    )
    .is_err());
    assert!(verify_report(&response(&id, &[]), &[], &id, &binding, &query).is_err());
}

#[test]
fn sealed_pair_reports_reject_extra_fields_other_variants_and_import_seal_adoption() {
    let (id, binding, query, report) = fixture();
    let mut corruptions = Vec::new();
    let mut extra = report.clone();
    extra["rendererPlane"] = json!({"normal":[0,0,1]});
    corruptions.push(extra);
    let mut extra_result = report.clone();
    extra_result["result"]["angleDeg"] = json!(0);
    corruptions.push(extra_result);
    let mut other_variant = report.clone();
    other_variant["result"] = json!({"kind":"faceNormalAngle","angleDeg":0});
    corruptions.push(other_variant);
    let mut imported = report.clone();
    imported["importedAssetSeals"] =
        json!([{"path":"assets/other.step","sha256":"c".repeat(64),"size":100}]);
    corruptions.push(imported);
    let mut missing_seals = report.clone();
    missing_seals
        .as_object_mut()
        .unwrap()
        .remove("importedAssetSeals");
    corruptions.push(missing_seals);
    let mut wrong_engine = report;
    wrong_engine["engine"]["name"] = json!("renderer");
    corruptions.push(wrong_engine);
    for forged in corruptions {
        let bytes = serde_json::to_vec(&forged).unwrap();
        // Recompute the transport hash so typed report validation must reject it.
        assert!(verify_report(&response(&id, &bytes), &bytes, &id, &binding, &query).is_err());
    }
}
