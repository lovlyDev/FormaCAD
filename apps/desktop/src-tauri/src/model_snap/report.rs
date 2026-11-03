//! Bounded verification of an isolated worker's exact-query envelope and source binding.
use super::schema::{SnapBinding, SnapQuery, SnapReport, MAX_REPORT_BYTES};
use crate::core::{AppError, Result};
use serde::Deserialize;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Response {
    protocol_version: u32,
    request_id: String,
    status: String,
    code: Option<String>,
    detail: Option<String>,
    volume_mm3: Option<f64>,
    area_mm2: Option<f64>,
    face_count: Option<i32>,
    edge_count: Option<i32>,
    bounds_mm: Option<[f64; 3]>,
    step_sha256: Option<String>,
    preview_sha256: Option<String>,
    section_sha256: Option<String>,
    measurement_sha256: Option<String>,
}
fn invalid(detail: impl Into<String>) -> AppError {
    AppError::Invalid(
        serde_json::json!({"code":"SNAP_REPORT_INVALID","message":detail.into()}).to_string(),
    )
}
pub fn verify_report(
    response: &[u8],
    report: &[u8],
    request_id: &str,
    binding: &SnapBinding,
    query: &SnapQuery,
) -> Result<SnapReport> {
    if response.is_empty()
        || response.len() > MAX_REPORT_BYTES
        || report.is_empty()
        || report.len() > MAX_REPORT_BYTES
    {
        return Err(invalid("Snap response/report exceeds bounds"));
    }
    let envelope: Response =
        serde_json::from_slice(response).map_err(|e| invalid(e.to_string()))?;
    if envelope.protocol_version != 1
        || envelope.request_id != request_id
        || envelope.status != "completed"
        || envelope.code.is_some()
        || envelope.detail.is_some()
        || envelope.volume_mm3.is_some()
        || envelope.area_mm2.is_some()
        || envelope.face_count.is_some()
        || envelope.edge_count.is_some()
        || envelope.bounds_mm.is_some()
        || envelope.step_sha256.is_some()
        || envelope.preview_sha256.is_some()
        || envelope.section_sha256.is_some()
        || envelope.measurement_sha256.as_deref() != Some(crate::artifacts::digest(report).as_str())
    {
        return Err(invalid(
            "Worker envelope did not exclusively match the snap request",
        ));
    }
    let report: SnapReport = serde_json::from_slice(report).map_err(|e| invalid(e.to_string()))?;
    if !report.matches(request_id, binding, query) {
        return Err(invalid(
            "Snap report did not match source, request and query",
        ));
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cad_ir::{TopologyKind, TopologyReference},
        model_snap::schema::{SnapEngine, SnapValue},
    };
    use serde_json::{json, Value};
    fn fixture() -> (String, SnapBinding, SnapQuery, Value) {
        let request = uuid::Uuid::new_v4().to_string();
        let binding = SnapBinding {
            revision_id: uuid::Uuid::new_v4().to_string(),
            body_id: "body".into(),
            source_sha256: "a".repeat(64),
            source_size: 120,
            document_sha256: "b".repeat(64),
            imported_asset_seals: vec![],
        };
        let query = SnapQuery::Face {
            reference: TopologyReference {
                schema_version: 1,
                kind: TopologyKind::Face,
                owner_feature_id: "pad".into(),
                role: "box-face:zmax".into(),
                occurrence_path: vec![],
            },
            ray_origin_mm: [0., 0., 100.],
            ray_direction: [0., 0., -1.],
        };
        let report = SnapReport {
            schema_version: 1,
            request_id: request.clone(),
            engine: SnapEngine {
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
            result: SnapValue {
                point_mm: [0., 0., 10.],
                normal: Some([0., 0., 1.]),
            },
        };
        (
            request,
            binding,
            query,
            serde_json::to_value(report).unwrap(),
        )
    }
    fn envelope(request: &str, bytes: &[u8]) -> Vec<u8> {
        serde_json::to_vec(&json!({"protocolVersion":1,"requestId":request,"status":"completed","measurementSha256":crate::artifacts::digest(bytes)})).unwrap()
    }
    #[test]
    fn recomputed_transport_seal_does_not_authorize_changed_source_or_ray() {
        let (request, binding, query, report) = fixture();
        let bytes = serde_json::to_vec(&report).unwrap();
        assert!(verify_report(
            &envelope(&request, &bytes),
            &bytes,
            &request,
            &binding,
            &query
        )
        .is_ok());
        for (field, value) in [
            ("bodyId", json!("other")),
            ("sourceSize", json!(121)),
            ("sourceSha256", json!("c".repeat(64))),
            ("documentSha256", json!("d".repeat(64))),
            ("revisionId", json!(uuid::Uuid::new_v4().to_string())),
            ("engine", json!({"name":"mesh","protocolVersion":1})),
            ("evaluationSource", json!("tessellation")),
        ] {
            let mut forged = report.clone();
            forged[field] = value;
            let bytes = serde_json::to_vec(&forged).unwrap();
            assert!(
                verify_report(
                    &envelope(&request, &bytes),
                    &bytes,
                    &request,
                    &binding,
                    &query
                )
                .is_err(),
                "accepted {field}"
            );
        }
        let mut changed = report;
        changed["query"]["rayOriginMm"] = json!([1., 0., 100.]);
        let bytes = serde_json::to_vec(&changed).unwrap();
        assert!(verify_report(
            &envelope(&request, &bytes),
            &bytes,
            &request,
            &binding,
            &query
        )
        .is_err());
    }
    #[test]
    fn wrong_result_shape_error_response_and_unexpected_outputs_are_rejected() {
        let (request, binding, query, report) = fixture();
        for result in [
            json!({"pointMm":[0.,0.,10.]}),
            json!({"pointMm":[0.,0.,10.],"normal":[0.,0.,2.]}),
            json!({"pointMm":[1e10,0.,10.],"normal":[0.,0.,1.]}),
        ] {
            let mut forged = report.clone();
            forged["result"] = result;
            let bytes = serde_json::to_vec(&forged).unwrap();
            assert!(verify_report(
                &envelope(&request, &bytes),
                &bytes,
                &request,
                &binding,
                &query
            )
            .is_err());
        }
        let bytes = serde_json::to_vec(&report).unwrap();
        let mut response: Value = serde_json::from_slice(&envelope(&request, &bytes)).unwrap();
        response["stepSha256"] = json!("a".repeat(64));
        assert!(verify_report(
            &serde_json::to_vec(&response).unwrap(),
            &bytes,
            &request,
            &binding,
            &query
        )
        .is_err());
        response["stepSha256"] = Value::Null;
        response["status"] = json!("error");
        assert!(verify_report(
            &serde_json::to_vec(&response).unwrap(),
            &bytes,
            &request,
            &binding,
            &query
        )
        .is_err());
    }
}
