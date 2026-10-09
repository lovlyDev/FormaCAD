#![cfg(feature = "native-occt")]
//! Genuine worker measurements of constructor-owned cylinders; private fixtures only.
use forma_core::{
    cad_ir::Document,
    model_measurement::schema::{MeasurementBinding, MeasurementQuery, MeasurementValue},
    native::{build_body, reference_measurements::client::measure_with_executable},
};
use serde_json::json;
use std::{
    path::Path,
    sync::{atomic::AtomicBool, Arc},
};

fn document(radius: f64, height: f64, transformed: bool) -> Document {
    let mut value = json!({"schemaVersion":2,"revisionId":uuid::Uuid::new_v4().to_string(),"parameters":[],"features":[
        {"id":"profile","name":"Profile","operation":{"type":"circle","radius":{"kind":"literal","mm":radius}}},
        {"id":"pad","name":"Pad","operation":{"type":"extrude","sketchId":"profile","distance":{"kind":"literal","mm":height}}},
        {"id":"other_pad","name":"Other","operation":{"type":"extrude","sketchId":"profile","distance":{"kind":"literal","mm":height}}}
    ],"bodies":[{"id":"body","name":"Body","sourceFeatureId":"pad"}]});
    if transformed {
        value["features"].as_array_mut().unwrap().extend([
            json!({"id":"turn","name":"Turn","operation":{"type":"rotate","bodyFeatureId":"pad","axisOriginMm":[0,0,0],"axisDirection":[0,1,0],"angleDeg":31}}),
            json!({"id":"reflect","name":"Mirror","operation":{"type":"mirror","bodyFeatureId":"turn","planeOriginMm":[7,0,0],"planeNormal":[1,0,0]}}),
        ]);
        value["bodies"][0]["sourceFeatureId"] = "reflect".into();
    }
    serde_json::from_value(value).unwrap()
}
fn query(kind: &str, role: &str, owner: &str, transformed: bool) -> MeasurementQuery {
    serde_json::from_value(json!({"kind":kind,"reference":{"schemaVersion":1,
        "kind":if kind.starts_with("edge") {"edge"} else {"face"},"ownerFeatureId":owner,
        "role":role,"occurrencePath":if transformed {json!(["turn","reflect"])} else {json!([])}}}))
    .unwrap()
}
fn staged(doc: &Document) -> (tempfile::TempDir, MeasurementBinding) {
    let directory = tempfile::tempdir().unwrap();
    let raw = serde_json::to_vec(doc).unwrap();
    std::fs::write(directory.path().join("document.json"), &raw).unwrap();
    build_body(doc, "body")
        .unwrap()
        .write_step(&directory.path().join("source.step"))
        .unwrap();
    let source = std::fs::read(directory.path().join("source.step")).unwrap();
    (
        directory,
        MeasurementBinding {
            revision_id: doc.revision_id.clone(),
            body_id: "body".into(),
            document_sha256: forma_core::artifacts::digest(&raw),
            source_sha256: forma_core::artifacts::digest(&source),
            source_size: source.len() as u64,
            imported_asset_seals: vec![],
        },
    )
}
async fn measure(
    directory: &Path,
    binding: &MeasurementBinding,
    query: &MeasurementQuery,
) -> forma_core::core::Result<MeasurementValue> {
    Ok(measure_with_executable(
        directory,
        Arc::new(AtomicBool::new(false)),
        Path::new(env!("CARGO_BIN_EXE_forma-cad-worker")),
        binding,
        query,
    )
    .await?
    .result)
}
fn close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-6, "{actual} != {expected}");
}

#[tokio::test]
async fn genuine_worker_measures_caps_after_resize_rotation_and_mirror_without_mutation() {
    for (radius, height, transformed) in [(10., 25., false), (17., 39., true)] {
        let (directory, binding) = staged(&document(radius, height, transformed));
        let source = std::fs::read(directory.path().join("source.step")).unwrap();
        let raw = std::fs::read(directory.path().join("document.json")).unwrap();
        for cap in ["bottom", "top"] {
            let role = format!("cylinder-edge:{cap}");
            for (kind, expected) in [
                ("edgeLength", 2. * std::f64::consts::PI * radius),
                ("edgeRadius", radius),
                ("edgeDiameter", 2. * radius),
            ] {
                let result = measure(
                    directory.path(),
                    &binding,
                    &query(kind, &role, "pad", transformed),
                )
                .await
                .unwrap();
                let actual = match result {
                    MeasurementValue::EdgeLength { length_mm } => length_mm,
                    MeasurementValue::EdgeRadius { radius_mm } => radius_mm,
                    MeasurementValue::EdgeDiameter { diameter_mm } => diameter_mm,
                    _ => panic!("Wrong scalar kind"),
                };
                close(actual, expected);
            }
            let face = format!("cylinder-face:{cap}");
            let MeasurementValue::PlanarFace {
                area_mm2,
                origin_mm,
                normal,
            } = measure(
                directory.path(),
                &binding,
                &query("planarFace", &face, "pad", transformed),
            )
            .await
            .unwrap()
            else {
                panic!("Wrong plane")
            };
            close(area_mm2, std::f64::consts::PI * radius * radius);
            let z = if cap == "top" { height } else { 0. };
            let sign = if cap == "top" { 1. } else { -1. };
            let angle = 31_f64.to_radians();
            let (origin, expected_normal) = if transformed {
                (
                    [14. - z * angle.sin(), 0., z * angle.cos()],
                    [-sign * angle.sin(), 0., sign * angle.cos()],
                )
            } else {
                ([0., 0., z], [0., 0., sign])
            };
            for (actual, expected) in origin_mm.into_iter().zip(origin) {
                close(actual, expected);
            }
            for (actual, expected) in normal.into_iter().zip(expected_normal) {
                close(actual, expected);
            }
        }
        let MeasurementValue::FaceArea { area_mm2 } = measure(
            directory.path(),
            &binding,
            &query("faceArea", "cylinder-face:side", "pad", transformed),
        )
        .await
        .unwrap() else {
            panic!("Wrong area")
        };
        close(area_mm2, 2. * std::f64::consts::PI * radius * height);
        assert_eq!(
            std::fs::read(directory.path().join("source.step")).unwrap(),
            source
        );
        assert_eq!(
            std::fs::read(directory.path().join("document.json")).unwrap(),
            raw
        );
        assert!(!directory.path().join("model.step").exists());
        assert!(!directory.path().join("preview.glb").exists());
    }
}

#[tokio::test]
async fn foreign_branch_curved_plane_and_negative_extrude_references_fail_explicitly() {
    for (height, owner, kind, role, expected) in [
        (
            25.,
            "other_pad",
            "edgeDiameter",
            "cylinder-edge:top",
            "TOPOLOGY_REFERENCE_UNRESOLVED",
        ),
        (
            25.,
            "pad",
            "planarFace",
            "cylinder-face:side",
            "MEASUREMENT_UNSUPPORTED",
        ),
        (
            -25.,
            "pad",
            "edgeRadius",
            "cylinder-edge:top",
            "TOPOLOGY_REFERENCE_UNSUPPORTED",
        ),
    ] {
        let (directory, binding) = staged(&document(10., height, false));
        let error = measure(directory.path(), &binding, &query(kind, role, owner, false))
            .await
            .unwrap_err();
        assert!(error.to_string().contains(expected), "{error}");
        assert!(!directory.path().join("measurement.json").exists());
    }
}

#[test]
fn circular_reference_is_not_a_certified_fillet_target() {
    let mut value = serde_json::to_value(document(10., 25., false)).unwrap();
    value["features"].as_array_mut().unwrap().push(json!({"id":"round","name":"Round","operation":{"type":"filletReferencedEdge","bodyFeatureId":"pad","radius":{"kind":"literal","mm":1},"reference":{"schemaVersion":1,"kind":"edge","ownerFeatureId":"pad","role":"cylinder-edge:top","occurrencePath":[]}}}));
    value["bodies"][0]["sourceFeatureId"] = "round".into();
    let doc: Document = serde_json::from_value(value).unwrap();
    assert!(doc.validate().is_err());
}
