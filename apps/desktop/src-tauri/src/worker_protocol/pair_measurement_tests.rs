//! Strict typed routing: reject renderer authority and cross-workflow field leakage.
use serde_json::{json, Value};

fn query() -> Value {
    let reference = |role: &str| {
        json!({"schemaVersion":1,"kind":"face",
        "ownerFeatureId":"pad","role":role,"occurrencePath":[]})
    };
    json!({"kind":"minimumDistance","first":reference("box-face:xmin"),
        "second":reference("box-face:xmax")})
}
fn bound_request(operation: &str) -> Value {
    json!({"protocolVersion":1,"requestId":uuid::Uuid::new_v4().to_string(),
        "operation":operation,"bodyId":"body","documentSha256":"a".repeat(64),
        "sourceSha256":"b".repeat(64),"sourceSize":100})
}
fn run(request: Value) -> Value {
    let stage = tempfile::tempdir().unwrap();
    super::run(
        serde_json::to_vec(&request).unwrap().as_slice(),
        stage.path(),
    )
    .unwrap();
    for file in [
        "pair-measurement.json",
        "measurement.json",
        "face-section.json",
        "section.json",
        "model.step",
        "preview.glb",
    ] {
        assert!(
            !stage.path().join(file).exists(),
            "Unexpected output {file}"
        );
    }
    serde_json::from_slice(&std::fs::read(stage.path().join("result.json")).unwrap()).unwrap()
}

#[test]
fn pair_operation_accepts_a_bound_typed_query_then_refuses_missing_fixed_inputs() {
    // This prevents all negative tests from passing merely because Request has
    // not registered pairMeasurementQuery or dispatch has no measure_pair route.
    let mut request = bound_request("measure_pair");
    request["pairMeasurementQuery"] = query();
    assert_eq!(run(request)["code"], "ASSET_MISSING");
}

#[test]
fn pair_query_never_leaks_into_existing_measurement_face_section_or_legacy_routes() {
    for operation in ["measure_reference", "section_reference"] {
        let mut request = bound_request(operation);
        request["pairMeasurementQuery"] = query();
        if operation == "measure_reference" {
            request["measurementQuery"] = json!({"kind":"bodyMetrics"});
        } else {
            request["faceSectionQuery"] = json!({"reference":query()["first"],
                "offsetMm":-5,"deflectionMm":0.01});
        }
        assert_eq!(run(request)["code"], "INVALID_REQUEST", "{operation}");
    }
    for operation in ["import_step", "inspect_step", "section_step"] {
        let mut request = json!({"protocolVersion":1,"requestId":"legacy",
            "operation":operation,"pairMeasurementQuery":query()});
        if operation == "section_step" {
            request["sourceSha256"] = "b".repeat(64).into();
            request["sectionPlane"] =
                json!({"originMm":[0,0,5],"normal":[0,0,1],"deflectionMm":0.01});
        }
        assert_eq!(run(request)["code"], "INVALID_REQUEST", "{operation}");
    }
    let request = json!({"protocolVersion":1,"requestId":"build","pairMeasurementQuery":query(),
        "document":{"schemaVersion":2,"revisionId":"candidate","parameters":[],"features":[],"bodies":[]}});
    assert_eq!(run(request)["code"], "INVALID_REQUEST");
}

#[test]
fn pair_requests_refuse_renderer_points_normals_scalars_paths_and_other_query_fields() {
    let mut base = bound_request("measure_pair");
    base["pairMeasurementQuery"] = query();
    for field in [
        "pointAMm",
        "normal",
        "distanceMm",
        "angleDeg",
        "ordinal",
        "secondBodyId",
        "sourcePath",
    ] {
        let mut request = base.clone();
        request["pairMeasurementQuery"][field] = json!([0, 0, 1]);
        assert_eq!(run(request)["code"], "INVALID_REQUEST", "query.{field}");
    }
    for field in ["ordinal", "pointMm", "normal"] {
        let mut request = base.clone();
        request["pairMeasurementQuery"]["second"][field] = 1.into();
        assert_eq!(run(request)["code"], "INVALID_REQUEST", "second.{field}");
    }
    for (field, value) in [
        ("measurementQuery", json!({"kind":"bodyMetrics"})),
        (
            "faceSectionQuery",
            json!({"reference":query()["first"],"offsetMm":0,"deflectionMm":0.01}),
        ),
        ("sectionPlane", json!({"originMm":[0,0,0],"normal":[0,0,1]})),
        (
            "document",
            json!({"schemaVersion":2,"revisionId":"candidate","parameters":[],"features":[],"bodies":[]}),
        ),
    ] {
        let mut request = base.clone();
        request[field] = value;
        assert_eq!(run(request)["code"], "INVALID_REQUEST", "{field}");
    }
}
