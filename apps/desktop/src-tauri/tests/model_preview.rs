#![cfg(feature = "native-occt")]
use forma_core::model_preview::build::{build_with_executable, validate_source};
use std::{
    path::Path,
    sync::{atomic::AtomicBool, Arc},
};
fn fixture() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../../docs/fixtures/sketch-holes.cad.json"
    ))
    .unwrap()
}
fn metrics(packet: &[u8]) -> serde_json::Value {
    let length = u32::from_le_bytes(packet[..4].try_into().unwrap()) as usize;
    assert_eq!(&packet[4 + length..8 + length], b"glTF");
    serde_json::from_slice(&packet[4..4 + length]).unwrap()
}
#[tokio::test]
async fn parameter_previews_are_exact_and_do_not_attach_geometry_or_modify_source() {
    let root = tempfile::tempdir().unwrap();
    let project_id = uuid::Uuid::new_v4().to_string();
    std::fs::create_dir(root.path().join(&project_id)).unwrap();
    let sentinel = root.path().join(&project_id).join("saved-history.json");
    std::fs::write(&sentinel, b"unchanged history").unwrap();
    let mut document = fixture();
    document["parameters"] = serde_json::json!([{"id":"height","name":"Height","valueMm":5}]);
    document["features"][1]["operation"]["distance"] =
        serde_json::json!({"kind":"parameter","parameterId":"height"});
    for (height, volume) in [(5, 5520.0), (10, 11040.0)] {
        document["parameters"][0]["valueMm"] = height.into();
        let source = document.to_string();
        let packet = build_with_executable(
            root.path(),
            &project_id,
            &source,
            Arc::new(AtomicBool::new(false)),
            Path::new(env!("CARGO_BIN_EXE_forma-cad-worker")),
        )
        .await
        .unwrap();
        let report = metrics(&packet);
        assert_eq!(report["protocolVersion"], 1);
        assert!((report["metrics"]["volumeMm3"].as_f64().unwrap() - volume).abs() < 0.001);
        assert_eq!(source, document.to_string());
        assert_eq!(
            std::fs::read_dir(root.path().join(".transient/previews"))
                .unwrap()
                .count(),
            0
        );
        assert!(!root.path().join(&project_id).join("cache").exists());
    }
    assert_eq!(std::fs::read(&sentinel).unwrap(), b"unchanged history");
    assert!(!root.path().join(&project_id).join("model.step").exists());
}
#[tokio::test]
async fn errors_and_cancellation_remove_only_temporary_workspaces() {
    let root = tempfile::tempdir().unwrap();
    let project_id = uuid::Uuid::new_v4().to_string();
    let mut document = fixture();
    document["features"][0]["operation"]["points"][4]["xMm"] = (-2).into();
    let error = build_with_executable(
        root.path(),
        &project_id,
        &document.to_string(),
        Arc::new(AtomicBool::new(false)),
        Path::new(env!("CARGO_BIN_EXE_forma-cad-worker")),
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("INVALID_SKETCH"));
    let error = build_with_executable(
        root.path(),
        &project_id,
        &fixture().to_string(),
        Arc::new(AtomicBool::new(true)),
        Path::new(env!("CARGO_BIN_EXE_forma-cad-worker")),
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("cancelled"));
    assert_eq!(
        std::fs::read_dir(root.path().join(".transient/previews"))
            .unwrap()
            .count(),
        0
    );
}
#[test]
fn unsupported_and_oversized_sources_are_rejected() {
    assert!(validate_source(&"x".repeat(60001)).is_err());
    assert!(validate_source("import cadquery as cq").is_err());
    assert!(validate_source("{\"version\":1,\"features\":[]}").is_err());
}
