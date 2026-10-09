use super::*;
use crate::{
    model_face_section::schema::{FaceSectionGeometry, ResolvedFace},
    model_measurement::schema::MeasurementEngine,
};
use serde_json::json;
fn fixture() -> (
    String,
    MeasurementBinding,
    FaceSectionQuery,
    FaceSectionReport,
) {
    let id = uuid::Uuid::new_v4().to_string();
    let binding = MeasurementBinding {
        revision_id: uuid::Uuid::new_v4().to_string(),
        body_id: "body".into(),
        source_sha256: "a".repeat(64),
        source_size: 100,
        document_sha256: "b".repeat(64),
        imported_asset_seals: vec![],
    };
    let query:FaceSectionQuery=serde_json::from_value(json!({"reference":{"schemaVersion":1,"kind":"face","ownerFeatureId":"pad","role":"box-face:zmax","occurrencePath":[]},"offsetMm":6,"deflectionMm":0.01})).unwrap();
    let face = ResolvedFace {
        area_mm2: 800.,
        origin_mm: [0., 0., 10.],
        normal: [0., 0., 1.],
    };
    let geometry = FaceSectionGeometry {
        plane: face.plane(&query),
        curves: vec![],
        total_length_mm: 0.,
    };
    let report = FaceSectionReport {
        schema_version: 1,
        request_id: id.clone(),
        engine: MeasurementEngine {
            name: "occt".into(),
            protocol_version: 1,
        },
        evaluation_source: "rebuiltAuthoredFaceAndSealedStep".into(),
        revision_id: binding.revision_id.clone(),
        body_id: binding.body_id.clone(),
        source_sha256: binding.source_sha256.clone(),
        source_size: binding.source_size,
        document_sha256: binding.document_sha256.clone(),
        imported_asset_seals: vec![],
        query: query.clone(),
        face,
        geometry,
    };
    (id, binding, query, report)
}
fn response(id: &str, bytes: &[u8]) -> Vec<u8> {
    serde_json::to_vec(&json!({"protocolVersion":1,"requestId":id,"status":"completed","sectionSha256":crate::artifacts::digest(bytes)})).unwrap()
}
#[test]
fn separately_sealed_section_still_requires_every_echo_and_derived_signed_plane() {
    let (id, binding, query, report) = fixture();
    let bytes = serde_json::to_vec(&report).unwrap();
    assert!(verify_report(&response(&id, &bytes), &bytes, &id, &binding, &query).is_ok());
    for field in [
        "bodyId",
        "revisionId",
        "documentSha256",
        "sourceSha256",
        "requestId",
        "evaluationSource",
    ] {
        let mut value = serde_json::to_value(&report).unwrap();
        value[field] = "wrong".into();
        let bytes = serde_json::to_vec(&value).unwrap();
        assert!(
            verify_report(&response(&id, &bytes), &bytes, &id, &binding, &query).is_err(),
            "{field}"
        );
    }
    for (field, value) in [
        ("originMm", json!([0, 0, 15])),
        ("normal", json!([0, 0, -1])),
    ] {
        let mut forged = serde_json::to_value(&report).unwrap();
        forged["geometry"]["plane"][field] = value;
        let bytes = serde_json::to_vec(&forged).unwrap();
        assert!(verify_report(&response(&id, &bytes), &bytes, &id, &binding, &query).is_err());
    }
    let mut contradictory: serde_json::Value =
        serde_json::from_slice(&response(&id, &bytes)).unwrap();
    contradictory["measurementSha256"] = "c".repeat(64).into();
    assert!(verify_report(
        &serde_json::to_vec(&contradictory).unwrap(),
        &bytes,
        &id,
        &binding,
        &query
    )
    .is_err());
}
