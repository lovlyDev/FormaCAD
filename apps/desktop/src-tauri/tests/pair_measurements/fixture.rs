use forma_core::{
    cad_ir::Document,
    model_measurement::schema::MeasurementBinding,
    model_pair_measurement::schema::{
        PairMeasurementQuery, PairMeasurementReport, PairMeasurementValue,
    },
    native::{build_body_with_assets, pair_measurements::client::measure_with_executable},
};
use serde_json::{json, Value};
use std::{
    path::Path,
    sync::{atomic::AtomicBool, Arc},
};

pub(super) fn document(round: bool, width: f64, height: f64, transformed: bool) -> Document {
    let profile = if round {
        json!({"type":"circle","radius":{"kind":"literal","mm":width}})
    } else {
        json!({"type":"rectangle","width":{"kind":"literal","mm":width},"depth":{"kind":"literal","mm":20}})
    };
    let mut value = json!({"schemaVersion":2,"revisionId":uuid::Uuid::new_v4().to_string(),"parameters":[],
        "features":[{"id":"profile","name":"Profile","operation":profile},
        {"id":"pad","name":"Pad","operation":{"type":"extrude","sketchId":"profile","distance":{"kind":"literal","mm":height}}},
        {"id":"foreign","name":"Other pad","operation":{"type":"extrude","sketchId":"profile","distance":{"kind":"literal","mm":height}}}],
        "bodies":[{"id":"body","name":"Body","sourceFeatureId":"pad"}]});
    if transformed {
        value["features"].as_array_mut().unwrap().extend([
            json!({"id":"turn","name":"Turn","operation":{"type":"rotate","bodyFeatureId":"pad","axisOriginMm":[0,0,0],"axisDirection":[0,1,0],"angleDeg":31}}),
            json!({"id":"shift","name":"Shift","operation":{"type":"translate","bodyFeatureId":"turn","offsetMm":[3,-4,9]}}),
            json!({"id":"reflect","name":"Mirror","operation":{"type":"mirror","bodyFeatureId":"shift","planeOriginMm":[7,0,0],"planeNormal":[1,0,0]}}),
        ]);
        value["bodies"][0]["sourceFeatureId"] = "reflect".into();
    }
    serde_json::from_value(value).unwrap()
}
pub(super) fn staged() -> (tempfile::TempDir, MeasurementBinding) {
    stage(&document(false, 40., 10., false), &[])
}
pub(super) fn stage(
    doc: &Document,
    imports: &[(String, Vec<u8>)],
) -> (tempfile::TempDir, MeasurementBinding) {
    let dir = tempfile::tempdir().unwrap();
    if !imports.is_empty() {
        std::fs::create_dir(dir.path().join("inputs")).unwrap();
    }
    for (hash, bytes) in imports {
        std::fs::write(
            dir.path()
                .join(forma_core::native::assets::relative_path(hash)),
            bytes,
        )
        .unwrap();
    }
    let raw = serde_json::to_vec(doc).unwrap();
    std::fs::write(dir.path().join("document.json"), &raw).unwrap();
    build_body_with_assets(doc, "body", Some(dir.path()))
        .unwrap()
        .write_step(&dir.path().join("source.step"))
        .unwrap();
    let source = std::fs::read(dir.path().join("source.step")).unwrap();
    let seals =
        forma_core::native::reference_measurements::execute::imported_seals(doc, dir.path())
            .unwrap();
    (
        dir,
        MeasurementBinding {
            revision_id: doc.revision_id.clone(),
            body_id: "body".into(),
            document_sha256: forma_core::artifacts::digest(&raw),
            source_sha256: forma_core::artifacts::digest(&source),
            source_size: source.len() as u64,
            imported_asset_seals: seals,
        },
    )
}
pub(super) fn reference(role: &str, transformed: bool) -> Value {
    json!({"schemaVersion":1,"kind":if role.contains("-face:") {"face"} else {"edge"},
        "ownerFeatureId":"pad","role":role,
        "occurrencePath":if transformed {json!(["turn","shift","reflect"])} else {json!([])}})
}
pub(super) fn pair(kind: &str, a: &str, b: &str, transformed: bool) -> PairMeasurementQuery {
    serde_json::from_value(
        json!({"kind":kind,"first":reference(a, transformed),"second":reference(b, transformed)}),
    )
    .unwrap()
}
pub(super) fn query(kind: &str, a: &str, b: &str) -> PairMeasurementQuery {
    pair(kind, a, b, false)
}
pub(super) async fn measure(
    path: &Path,
    binding: &MeasurementBinding,
    q: &PairMeasurementQuery,
) -> forma_core::core::Result<PairMeasurementReport> {
    measure_with_executable(
        path,
        Arc::new(AtomicBool::new(false)),
        Path::new(env!("CARGO_BIN_EXE_forma-cad-worker")),
        binding,
        q,
    )
    .await
}
pub(super) fn scalar(value: &PairMeasurementValue) -> f64 {
    match value {
        PairMeasurementValue::MinimumDistance { distance_mm, .. } => *distance_mm,
        PairMeasurementValue::FaceNormalAngle { angle_deg }
        | PairMeasurementValue::EdgeAcuteAngle { angle_deg } => *angle_deg,
    }
}
pub(super) fn close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-6, "{actual} != {expected}");
}
pub(super) fn no_geometry(path: &Path) {
    for file in [
        "model.step",
        "preview.glb",
        "measurement.json",
        "section.json",
        "face-section.json",
    ] {
        assert!(!path.join(file).exists(), "Unexpected {file}");
    }
}
