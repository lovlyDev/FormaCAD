#![cfg(feature = "native-occt")]

use std::{
    io::Write,
    process::{Command, Stdio},
};

#[test]
fn isolated_worker_builds_step_and_reports_measurements() {
    let cwd = tempfile::tempdir().unwrap();
    let document = serde_json::json!({
        "schemaVersion": 2,
        "revisionId": "revision_1",
        "parameters": [],
        "features": [
            {"id":"profile","name":"Profile","operation":{"type":"rectangle","width":{"kind":"literal","mm":10.0},"depth":{"kind":"literal","mm":20.0}}},
            {"id":"pad","name":"Pad","operation":{"type":"extrude","sketchId":"profile","distance":{"kind":"literal","mm":5.0}}}
        ],
        "bodies": [{"id":"body","name":"Plate","sourceFeatureId":"pad"}]
    });
    let input = serde_json::json!({
        "protocolVersion": 1,
        "requestId": "test_request",
        "bodyId": "body",
        "document": document
    });
    let mut child = Command::new(env!("CARGO_BIN_EXE_forma-cad-worker"))
        .current_dir(cwd.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.to_string().as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result: serde_json::Value =
        serde_json::from_slice(&std::fs::read(cwd.path().join("result.json")).unwrap()).unwrap();
    assert_eq!(result["status"], "completed", "{result}");
    assert_eq!(result["requestId"], "test_request");
    assert!((result["volumeMm3"].as_f64().unwrap() - 1000.0).abs() < 0.001);
    assert_eq!(result["faceCount"], 6);
    assert!(cwd.path().join("model.step").metadata().unwrap().len() > 0);
    assert!(cwd.path().join("preview.glb").metadata().unwrap().len() > 100);
    assert_eq!(result["previewSha256"].as_str().unwrap().len(), 64);
}

#[tokio::test]
async fn constrained_sketch_builds_in_worker_and_conflict_leaves_no_artifacts() {
    let document = serde_json::json!({
        "schemaVersion":2,"revisionId":"sketch_worker","parameters":[],
        "features":[
            {"id":"profile","name":"Triangle","operation":{
                "type":"sketch2d","plane":"yz","originMm":[4.0,0.0,0.0],
                "points":[
                    {"id":"a","xMm":0.0,"yMm":0.0},
                    {"id":"b","xMm":20.0,"yMm":0.0},
                    {"id":"c","xMm":0.0,"yMm":10.0}
                ],
                "lines":[
                    {"id":"ab","startPointId":"a","endPointId":"b"},
                    {"id":"bc","startPointId":"b","endPointId":"c"},
                    {"id":"ca","startPointId":"c","endPointId":"a"}
                ],
                "constraints":[{"kind":"horizontal","id":"h","lineId":"ab"}]
            }},
            {"id":"pad","name":"Pad","operation":{"type":"extrude","sketchId":"profile","distance":{"kind":"literal","mm":5.0}}}
        ],
        "bodies":[{"id":"body","name":"Body","sourceFeatureId":"pad"}]
    });
    let worker = std::path::Path::new(env!("CARGO_BIN_EXE_forma-cad-worker"));
    let valid_dir = tempfile::tempdir().unwrap();
    forma_core::native::worker::build_with_executable(
        &document.to_string(),
        valid_dir.path(),
        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        worker,
    )
    .await
    .unwrap();
    assert!(valid_dir.path().join("model.step").is_file());
    assert!(valid_dir.path().join("preview.glb").is_file());
    let mut invalid = document;
    let constraints = invalid["features"][0]["operation"]["constraints"]
        .as_array_mut()
        .unwrap();
    constraints.push(serde_json::json!({"kind":"length","id":"first","lineId":"ab","distance":{"kind":"literal","mm":20.0}}));
    constraints.push(serde_json::json!({"kind":"length","id":"second","lineId":"ab","distance":{"kind":"literal","mm":30.0}}));
    let invalid_dir = tempfile::tempdir().unwrap();
    let error = forma_core::native::worker::build_with_executable(
        &invalid.to_string(),
        invalid_dir.path(),
        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        worker,
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("SKETCH_CONSTRAINT_CONFLICT"));
    assert!(!invalid_dir.path().join("model.step").exists());
    assert!(!invalid_dir.path().join("preview.glb").exists());
}

#[tokio::test]
async fn app_client_validates_worker_artifacts() {
    let cwd = tempfile::tempdir().unwrap();
    let document = serde_json::json!({
        "schemaVersion": 2,
        "revisionId": "revision_2",
        "parameters": [],
        "features": [
            {"id":"profile","name":"Profile","operation":{"type":"circle","radius":{"kind":"literal","mm":5.0}}},
            {"id":"pad","name":"Pad","operation":{"type":"extrude","sketchId":"profile","distance":{"kind":"literal","mm":10.0}}}
        ],
        "bodies": [{"id":"body","name":"Cylinder","sourceFeatureId":"pad"}]
    });
    let worker = std::path::Path::new(env!("CARGO_BIN_EXE_forma-cad-worker"));
    let built = forma_core::native::worker::build_with_executable(
        &document.to_string(),
        cwd.path(),
        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        worker,
    )
    .await
    .unwrap();
    assert!(built);
    assert!(cwd.path().join("model.step").is_file());
    assert!(cwd.path().join("preview.glb").is_file());
    let glb = std::fs::read(cwd.path().join("preview.glb")).unwrap();
    let json_length = u32::from_le_bytes(glb[12..16].try_into().unwrap()) as usize;
    let metadata: serde_json::Value = serde_json::from_slice(&glb[20..20 + json_length]).unwrap();
    let edges = metadata["meshes"][0]["primitives"][0]["extras"]["formaEdges"]
        .as_array()
        .unwrap();
    assert_eq!(edges.len(), 3);
    let straight = edges
        .iter()
        .filter(|edge| (edge["lengthMm"].as_f64().unwrap() - 10.0).abs() < 0.001)
        .count();
    let circular = edges
        .iter()
        .filter(|edge| {
            (edge["lengthMm"].as_f64().unwrap() - 10.0 * std::f64::consts::PI).abs() < 0.001
        })
        .count();
    assert_eq!((straight, circular), (1, 2));
    assert_eq!(
        edges
            .iter()
            .filter(|edge| edge["radiusMm"]
                .as_f64()
                .is_some_and(|radius| (radius - 5.0).abs() < 0.001))
            .count(),
        2
    );
    let areas = metadata["meshes"][0]["primitives"][0]["extras"]["formaFaceAreasMm2"]
        .as_array()
        .unwrap();
    let total: f64 = areas.iter().map(|value| value.as_f64().unwrap()).sum();
    assert!((total - 150.0 * std::f64::consts::PI).abs() < 0.001);
}

#[tokio::test]
async fn inspection_reports_exact_brep_properties_without_exporting_a_preview() {
    let cwd = tempfile::tempdir().unwrap();
    forma_core::native::Solid::box_solid(60.0, 40.0, 10.0)
        .unwrap()
        .write_step(&cwd.path().join("source.step"))
        .unwrap();
    let properties = forma_core::native::inspection::inspect_with_executable(
        cwd.path(),
        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        std::path::Path::new(env!("CARGO_BIN_EXE_forma-cad-worker")),
    )
    .await
    .unwrap();
    assert!((properties.volume_mm3 - 24_000.0).abs() < 0.001);
    assert!((properties.area_mm2 - 6_800.0).abs() < 0.001);
    assert_eq!((properties.face_count, properties.edge_count), (6, 12));
    for (actual, expected) in properties.bounds_mm.into_iter().zip([60.0, 40.0, 10.0]) {
        assert!((actual - expected).abs() < 0.001);
    }
    assert!(!cwd.path().join("preview.glb").exists());
}

#[tokio::test]
async fn inspection_counts_separate_bodies_in_a_step_assembly() {
    let cwd = tempfile::tempdir().unwrap();
    let first = forma_core::native::Solid::box_solid(10.0, 10.0, 10.0).unwrap();
    let second = first.translated(30.0, 0.0, 0.0).unwrap();
    first
        .compound(&second)
        .unwrap()
        .write_step(&cwd.path().join("source.step"))
        .unwrap();
    let properties = forma_core::native::inspection::inspect_with_executable(
        cwd.path(),
        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        std::path::Path::new(env!("CARGO_BIN_EXE_forma-cad-worker")),
    )
    .await
    .unwrap();
    assert!((properties.volume_mm3 - 2_000.0).abs() < 0.001);
    assert!((properties.area_mm2 - 1_200.0).abs() < 0.001);
    assert_eq!((properties.face_count, properties.edge_count), (12, 24));
    for (actual, expected) in properties.bounds_mm.into_iter().zip([40.0, 10.0, 10.0]) {
        assert!((actual - expected).abs() < 0.001);
    }
}

#[tokio::test]
async fn rejected_geometry_leaves_no_exported_model() {
    let cwd = tempfile::tempdir().unwrap();
    let document = serde_json::json!({
        "schemaVersion": 2,
        "revisionId": "revision_invalid",
        "parameters": [],
        "features": [
            {"id":"profile","name":"Profile","operation":{"type":"rectangle","width":{"kind":"literal","mm":-10.0},"depth":{"kind":"literal","mm":20.0}}},
            {"id":"pad","name":"Pad","operation":{"type":"extrude","sketchId":"profile","distance":{"kind":"literal","mm":5.0}}}
        ],
        "bodies": [{"id":"body","name":"Invalid","sourceFeatureId":"pad"}]
    });
    let error = forma_core::native::worker::build_with_executable(
        &document.to_string(),
        cwd.path(),
        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        std::path::Path::new(env!("CARGO_BIN_EXE_forma-cad-worker")),
    )
    .await;
    assert!(error.is_err());
    assert!(!cwd.path().join("model.step").exists());
    assert!(!cwd.path().join("preview.glb").exists());
}

#[tokio::test]
async fn two_named_bodies_survive_step_and_glb_export() {
    let cwd = tempfile::tempdir().unwrap();
    let document = serde_json::json!({
        "schemaVersion": 2,
        "revisionId": "revision_multi",
        "parameters": [],
        "features": [
            {"id":"housing_profile","name":"Housing profile","operation":{"type":"rectangle","width":{"kind":"literal","mm":10.0},"depth":{"kind":"literal","mm":20.0}}},
            {"id":"housing_pad","name":"Housing pad","operation":{"type":"extrude","sketchId":"housing_profile","distance":{"kind":"literal","mm":5.0}}},
            {"id":"lid_profile","name":"Lid profile","operation":{"type":"rectangle","width":{"kind":"literal","mm":10.0},"depth":{"kind":"literal","mm":20.0}}},
            {"id":"lid_pad","name":"Lid pad","operation":{"type":"extrude","sketchId":"lid_profile","distance":{"kind":"literal","mm":2.0}}},
            {"id":"lid_move","name":"Lid offset","operation":{"type":"translate","bodyFeatureId":"lid_pad","offsetMm":[25.0,0.0,0.0]}}
        ],
        "bodies": [
            {"id":"housing","name":"Housing","sourceFeatureId":"housing_pad"},
            {"id":"lid","name":"Lid","sourceFeatureId":"lid_move"}
        ]
    });
    forma_core::native::worker::build_with_executable(
        &document.to_string(),
        cwd.path(),
        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        std::path::Path::new(env!("CARGO_BIN_EXE_forma-cad-worker")),
    )
    .await
    .unwrap();
    let result: serde_json::Value =
        serde_json::from_slice(&std::fs::read(cwd.path().join("result.json")).unwrap()).unwrap();
    assert!((result["volumeMm3"].as_f64().unwrap() - 1400.0).abs() < 0.001);
    let imported = forma_core::native::Solid::read_step(&cwd.path().join("model.step")).unwrap();
    assert!((imported.volume_mm3() - 1400.0).abs() < 0.001);
    let glb = std::fs::read(cwd.path().join("preview.glb")).unwrap();
    let json_length = u32::from_le_bytes(glb[12..16].try_into().unwrap()) as usize;
    let metadata: serde_json::Value = serde_json::from_slice(&glb[20..20 + json_length]).unwrap();
    assert_eq!(metadata["nodes"].as_array().unwrap().len(), 2);
    assert_eq!(metadata["meshes"].as_array().unwrap().len(), 2);
    assert_eq!(metadata["nodes"][0]["extras"]["formaBodyId"], "housing");
    assert_eq!(metadata["nodes"][1]["extras"]["formaBodyId"], "lid");
    for (body_index, mesh) in metadata["meshes"].as_array().unwrap().iter().enumerate() {
        let counts = mesh["primitives"][0]["extras"]["formaFaceTriangleCounts"]
            .as_array()
            .unwrap();
        let areas = mesh["primitives"][0]["extras"]["formaFaceAreasMm2"]
            .as_array()
            .unwrap();
        let edges = mesh["primitives"][0]["extras"]["formaEdges"]
            .as_array()
            .unwrap();
        assert_eq!(edges.len(), 12);
        assert!(edges
            .iter()
            .all(|edge| edge["lengthMm"].as_f64().unwrap() > 0.0));
        assert_eq!(counts.len(), 6);
        assert_eq!(areas.len(), counts.len());
        let triangles: u64 = counts.iter().map(|count| count.as_u64().unwrap()).sum();
        let index_accessor = mesh["primitives"][0]["indices"].as_u64().unwrap() as usize;
        assert_eq!(
            triangles * 3,
            metadata["accessors"][index_accessor]["count"]
                .as_u64()
                .unwrap()
        );
        let area: f64 = areas.iter().map(|value| value.as_f64().unwrap()).sum();
        let expected = if body_index == 0 { 700.0 } else { 520.0 };
        assert!((area - expected).abs() < 0.001);
    }
}

#[tokio::test]
async fn selected_body_export_contains_only_requested_solid() {
    let cwd = tempfile::tempdir().unwrap();
    let document = serde_json::json!({
        "schemaVersion": 2,
        "revisionId": "revision_selection",
        "parameters": [],
        "features": [
            {"id":"first_profile","name":"First profile","operation":{"type":"rectangle","width":{"kind":"literal","mm":10.0},"depth":{"kind":"literal","mm":20.0}}},
            {"id":"first_pad","name":"First pad","operation":{"type":"extrude","sketchId":"first_profile","distance":{"kind":"literal","mm":5.0}}},
            {"id":"second_profile","name":"Second profile","operation":{"type":"circle","radius":{"kind":"literal","mm":5.0}}},
            {"id":"second_pad","name":"Second pad","operation":{"type":"extrude","sketchId":"second_profile","distance":{"kind":"literal","mm":10.0}}}
        ],
        "bodies": [
            {"id":"housing","name":"Housing","sourceFeatureId":"first_pad"},
            {"id":"shaft","name":"Shaft","sourceFeatureId":"second_pad"}
        ]
    });
    let worker = std::path::Path::new(env!("CARGO_BIN_EXE_forma-cad-worker"));
    let built = forma_core::native::worker::build_selected_with_executable(
        &document.to_string(),
        "shaft",
        cwd.path(),
        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        worker,
    )
    .await
    .unwrap();
    assert!(built);
    let shape = forma_core::native::Solid::read_step(&cwd.path().join("model.step")).unwrap();
    assert!((shape.volume_mm3() - std::f64::consts::PI * 250.0).abs() < 0.01);
    let glb = std::fs::read(cwd.path().join("preview.glb")).unwrap();
    let json_length = u32::from_le_bytes(glb[12..16].try_into().unwrap()) as usize;
    let metadata: serde_json::Value = serde_json::from_slice(&glb[20..20 + json_length]).unwrap();
    assert_eq!(metadata["nodes"].as_array().unwrap().len(), 1);
    assert_eq!(metadata["nodes"][0]["extras"]["formaBodyId"], "shaft");
}

#[tokio::test]
async fn imported_step_is_previewed_without_python() {
    let cwd = tempfile::tempdir().unwrap();
    forma_core::native::Solid::box_solid(12.0, 20.0, 5.0)
        .unwrap()
        .write_step(&cwd.path().join("source.step"))
        .unwrap();
    forma_core::native::worker::import_step_with_executable(
        cwd.path(),
        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        std::path::Path::new(env!("CARGO_BIN_EXE_forma-cad-worker")),
    )
    .await
    .unwrap();
    let imported = forma_core::native::Solid::read_step(&cwd.path().join("model.step")).unwrap();
    assert!((imported.volume_mm3() - 1200.0).abs() < 0.001);
    assert!(cwd.path().join("preview.glb").metadata().unwrap().len() > 100);
}

#[tokio::test]
async fn malformed_step_does_not_produce_a_preview() {
    let cwd = tempfile::tempdir().unwrap();
    std::fs::write(cwd.path().join("source.step"), b"not a STEP file").unwrap();
    let result = forma_core::native::worker::import_step_with_executable(
        cwd.path(),
        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        std::path::Path::new(env!("CARGO_BIN_EXE_forma-cad-worker")),
    )
    .await;
    assert!(result.is_err());
    assert!(!cwd.path().join("preview.glb").exists());
}

#[tokio::test]
async fn sphere_and_cone_are_native_typed_features() {
    let cwd = tempfile::tempdir().unwrap();
    let document = serde_json::json!({
        "schemaVersion": 2,
        "revisionId": "revision_curved",
        "parameters": [{"id":"radius","name":"Radius","valueMm":5.0}],
        "features": [
            {"id":"ball","name":"Ball","operation":{"type":"sphere","radius":{"kind":"parameter","parameterId":"radius"}}},
            {"id":"point","name":"Point","operation":{"type":"cone","bottomRadius":{"kind":"parameter","parameterId":"radius"},"topRadius":{"kind":"literal","mm":0.0},"height":{"kind":"literal","mm":10.0}}},
            {"id":"point_move","name":"Move point","operation":{"type":"translate","bodyFeatureId":"point","offsetMm":[20.0,0.0,0.0]}}
        ],
        "bodies": [
            {"id":"ball_body","name":"Ball","sourceFeatureId":"ball"},
            {"id":"point_body","name":"Point","sourceFeatureId":"point_move"}
        ]
    });
    forma_core::native::worker::build_with_executable(
        &document.to_string(),
        cwd.path(),
        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        std::path::Path::new(env!("CARGO_BIN_EXE_forma-cad-worker")),
    )
    .await
    .unwrap();
    let shape = forma_core::native::Solid::read_step(&cwd.path().join("model.step")).unwrap();
    let expected = std::f64::consts::PI * (500.0 / 3.0 + 250.0 / 3.0);
    assert!((shape.volume_mm3() - expected).abs() < 0.01);
}
