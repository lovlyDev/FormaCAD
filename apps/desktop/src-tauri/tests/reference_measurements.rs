#![cfg(feature = "native-occt")]
//! Real isolated worker, exact reconstructed body and analytic values. Stage only.
use forma_core::{
    cad_ir::Document,
    model_measurement::schema::{MeasurementBinding, MeasurementQuery, MeasurementValue},
    native::{build_body, reference_measurements::client::measure_with_executable},
};
use serde_json::{json, Value};
use std::{
    path::Path,
    sync::{atomic::AtomicBool, Arc},
};

fn document(transformed: bool) -> Document {
    let mut value = json!({"schemaVersion":2,"revisionId":uuid::Uuid::new_v4().to_string(),"parameters":[],
        "features":[{"id":"profile","name":"Profile","operation":{"type":"rectangle","width":{"kind":"literal","mm":40},"depth":{"kind":"literal","mm":20}}},
        {"id":"pad","name":"Pad","operation":{"type":"extrude","sketchId":"profile","distance":{"kind":"literal","mm":10}}},
        {"id":"other_pad","name":"Other","operation":{"type":"extrude","sketchId":"profile","distance":{"kind":"literal","mm":10}}}],
        "bodies":[{"id":"body","name":"Body","sourceFeatureId":"pad"}]});
    if transformed {
        value["features"].as_array_mut().unwrap().extend([
            json!({"id":"turn","name":"Turn","operation":{"type":"rotate","bodyFeatureId":"pad","axisOriginMm":[0,0,0],"axisDirection":[0,0,1],"angleDeg":17}}),
            json!({"id":"reflect","name":"Mirror","operation":{"type":"mirror","bodyFeatureId":"turn","planeOriginMm":[7,0,0],"planeNormal":[1,0,0]}}),
        ]);
        value["bodies"][0]["sourceFeatureId"] = "reflect".into();
    }
    serde_json::from_value(value).unwrap()
}

fn query(kind: &str, owner: &str, path: Value, role: &str) -> MeasurementQuery {
    serde_json::from_value(json!({"kind":kind,"reference":{"schemaVersion":1,
        "kind":if kind.starts_with("edge") { "edge" } else { "face" },
        "ownerFeatureId":owner,"role":role,"occurrencePath":path}}))
    .unwrap()
}
fn staged(document: &Document) -> (tempfile::TempDir, MeasurementBinding) {
    let directory = tempfile::tempdir().unwrap();
    let raw = serde_json::to_vec(document).unwrap();
    std::fs::write(directory.path().join("document.json"), &raw).unwrap();
    build_body(document, "body")
        .unwrap()
        .write_step(&directory.path().join("source.step"))
        .unwrap();
    let bytes = std::fs::read(directory.path().join("source.step")).unwrap();
    let binding = MeasurementBinding {
        revision_id: document.revision_id.clone(),
        body_id: "body".into(),
        document_sha256: forma_core::artifacts::digest(&raw),
        source_sha256: forma_core::artifacts::digest(&bytes),
        source_size: bytes.len() as u64,
        imported_asset_seals: vec![],
    };
    (directory, binding)
}
async fn measure(
    directory: &Path,
    binding: &MeasurementBinding,
    query: &MeasurementQuery,
) -> forma_core::core::Result<forma_core::model_measurement::schema::MeasurementReport> {
    measure_with_executable(
        directory,
        Arc::new(AtomicBool::new(false)),
        Path::new(env!("CARGO_BIN_EXE_forma-cad-worker")),
        binding,
        query,
    )
    .await
}

#[tokio::test]
async fn worker_body_edge_and_face_values_are_exact_and_sources_remain_unchanged() {
    let doc = document(false);
    let (directory, binding) = staged(&doc);
    let before = std::fs::read(directory.path().join("source.step")).unwrap();
    let body = measure(
        directory.path(),
        &binding,
        &MeasurementQuery::BodyMetrics {},
    )
    .await
    .unwrap();
    let MeasurementValue::BodyMetrics {
        volume_mm3,
        area_mm2,
        extents_mm,
        ..
    } = body.result
    else {
        panic!("wrong kind")
    };
    assert!((volume_mm3 - 8000.).abs() < 1e-7);
    assert!((area_mm2 - 2800.).abs() < 1e-7);
    for (actual, expected) in extents_mm.into_iter().zip([40., 20., 10.]) {
        assert!((actual - expected).abs() < 1e-6);
    }
    let length = measure(
        directory.path(),
        &binding,
        &query("edgeLength", "pad", json!([]), "box-edge:x:ymin:zmax"),
    )
    .await
    .unwrap();
    assert_eq!(
        length.result,
        MeasurementValue::EdgeLength { length_mm: 40. }
    );
    let area = measure(
        directory.path(),
        &binding,
        &query("faceArea", "pad", json!([]), "box-face:zmax"),
    )
    .await
    .unwrap();
    let MeasurementValue::FaceArea { area_mm2 } = area.result else {
        panic!("wrong kind")
    };
    assert!((area_mm2 - 800.).abs() < 1e-7);
    assert_eq!(
        std::fs::read(directory.path().join("source.step")).unwrap(),
        before
    );
    assert!(!directory.path().join("model.step").exists());
    assert!(!directory.path().join("preview.glb").exists());
}

#[tokio::test]
async fn oriented_plane_survives_rotation_and_nonzero_mirror_without_normal_sign_loss() {
    let (directory, binding) = staged(&document(true));
    let plane = measure(
        directory.path(),
        &binding,
        &query(
            "planarFace",
            "pad",
            json!(["turn", "reflect"]),
            "box-face:xmin",
        ),
    )
    .await
    .unwrap();
    let MeasurementValue::PlanarFace {
        origin_mm,
        normal,
        area_mm2,
    } = plane.result
    else {
        panic!("wrong kind")
    };
    let theta = 17_f64.to_radians();
    let expected_origin = [14. + 20. * theta.cos(), -20. * theta.sin(), 5.];
    let expected_normal = [theta.cos(), -theta.sin(), 0.];
    for (actual, expected) in origin_mm.into_iter().zip(expected_origin) {
        assert!((actual - expected).abs() < 1e-7);
    }
    for (actual, expected) in normal.into_iter().zip(expected_normal) {
        assert!((actual - expected).abs() < 1e-9);
    }
    assert!((area_mm2 - 200.).abs() < 1e-7);
}

#[tokio::test]
async fn similar_foreign_branch_and_straight_edge_radius_are_explicit_errors() {
    let (directory, binding) = staged(&document(false));
    let wrong = measure(
        directory.path(),
        &binding,
        &query("edgeLength", "other_pad", json!([]), "box-edge:x:ymin:zmax"),
    )
    .await
    .unwrap_err();
    assert!(wrong.to_string().contains("TOPOLOGY_REFERENCE_UNRESOLVED"));
    let radius = measure(
        directory.path(),
        &binding,
        &query("edgeRadius", "pad", json!([]), "box-edge:x:ymin:zmax"),
    )
    .await
    .unwrap_err();
    assert!(radius.to_string().contains("MEASUREMENT_UNSUPPORTED"));
    assert!(!directory.path().join("measurement.json").exists());
}

#[tokio::test]
async fn imported_body_metrics_echo_verified_asset_and_corrupt_staged_input_fails_closed() {
    let source_directory = tempfile::tempdir().unwrap();
    let source_path = source_directory.path().join("import.step");
    forma_core::native::Solid::box_solid(40., 20., 10.)
        .unwrap()
        .write_step(&source_path)
        .unwrap();
    let source = std::fs::read(source_path).unwrap();
    let hash = forma_core::artifacts::digest(&source);
    let asset_id = format!("step_{hash}");
    let doc: Document = serde_json::from_value(json!({"schemaVersion":2,
        "revisionId":uuid::Uuid::new_v4().to_string(),"parameters":[],
        "features":[{"id":"imported","name":"Imported","operation":{"type":"importStep",
            "assetId":asset_id,"sha256":hash}}],
        "bodies":[{"id":"body","name":"Body","sourceFeatureId":"imported"}]}))
    .unwrap();
    let directory = tempfile::tempdir().unwrap();
    std::fs::create_dir(directory.path().join("inputs")).unwrap();
    let asset_path = directory
        .path()
        .join(forma_core::native::assets::relative_path(&hash));
    std::fs::write(&asset_path, &source).unwrap();
    std::fs::write(directory.path().join("source.step"), &source).unwrap();
    let raw = serde_json::to_vec(&doc).unwrap();
    std::fs::write(directory.path().join("document.json"), &raw).unwrap();
    let binding = MeasurementBinding {
        revision_id: doc.revision_id,
        body_id: "body".into(),
        source_sha256: hash.clone(),
        source_size: source.len() as u64,
        document_sha256: forma_core::artifacts::digest(&raw),
        imported_asset_seals: vec![forma_core::model_measurement::schema::ImportedAssetSeal {
            asset_id,
            sha256: hash,
            size: source.len() as u64,
        }],
    };
    let report = measure(
        directory.path(),
        &binding,
        &MeasurementQuery::BodyMetrics {},
    )
    .await
    .unwrap();
    let MeasurementValue::BodyMetrics { volume_mm3, .. } = report.result else {
        panic!("wrong kind")
    };
    assert!((volume_mm3 - 8000.).abs() < 1e-7);
    assert_eq!(report.imported_asset_seals, binding.imported_asset_seals);
    let unsupported = measure(
        directory.path(),
        &binding,
        &query("edgeLength", "imported", json!([]), "box-edge:x:ymin:zmax"),
    )
    .await
    .unwrap_err();
    assert!(unsupported
        .to_string()
        .contains("TOPOLOGY_REFERENCE_UNSUPPORTED"));
    let previous_report = std::fs::read(directory.path().join("measurement.json")).unwrap();
    let mut corrupted = source.clone();
    corrupted[0] ^= 1;
    std::fs::write(&asset_path, corrupted).unwrap();
    let error = measure(
        directory.path(),
        &binding,
        &MeasurementQuery::BodyMetrics {},
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("ASSET_CHECKSUM_MISMATCH"));
    assert_eq!(
        std::fs::read(directory.path().join("source.step")).unwrap(),
        source
    );
    assert_eq!(
        std::fs::read(directory.path().join("measurement.json")).unwrap(),
        previous_report,
        "Failure must not publish a new report; an old temporary report is never authorized"
    );
    assert!(!directory.path().join("model.step").exists());
    assert!(!directory.path().join("preview.glb").exists());
}
