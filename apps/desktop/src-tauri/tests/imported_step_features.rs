#![cfg(feature = "native-occt")]
use forma_core::{
    cad_ir::{apply_commands, Command, Document, ErrorCode},
    native::{build_body, build_body_with_assets, Solid},
};
use serde_json::json;
use std::{
    path::Path,
    sync::{atomic::AtomicBool, Arc},
};

fn source(directory: &Path) -> (Vec<u8>, String) {
    let path = directory.join("original.step");
    Solid::box_solid(40., 20., 10.)
        .unwrap()
        .write_step(&path)
        .unwrap();
    let bytes = std::fs::read(path).unwrap();
    let hash = forma_core::artifacts::digest(&bytes);
    (bytes, hash)
}
fn document(hash: &str) -> Document {
    serde_json::from_value(json!({"schemaVersion":2,"revisionId":"import_base","parameters":[],"features":[
        {"id":"imported","name":"Imported STEP","operation":{"type":"importStep","assetId":format!("step_{hash}"),"sha256":hash}},
        {"id":"moved","name":"Moved","operation":{"type":"translate","bodyFeatureId":"imported","offsetMm":[10,0,0]}},
        {"id":"tool_profile","name":"Hole profile","operation":{"type":"circle","radius":{"kind":"literal","mm":2}}},
        {"id":"tool_pad","name":"Hole tool","operation":{"type":"extrude","sketchId":"tool_profile","distance":{"kind":"literal","mm":10}}},
        {"id":"tool","name":"Moved tool","operation":{"type":"translate","bodyFeatureId":"tool_pad","offsetMm":[10,0,0]}},
        {"id":"cut","name":"Cut","operation":{"type":"boolean","leftFeatureId":"moved","rightFeatureId":"tool","mode":"cut"}}
    ],"bodies":[{"id":"body","name":"Body","sourceFeatureId":"cut"}]})).unwrap()
}
fn stage(directory: &Path, hash: &str, bytes: &[u8]) {
    forma_core::artifacts::immutable_write(
        directory,
        &forma_core::native::assets::relative_path(hash),
        bytes,
    )
    .unwrap();
}
fn close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < expected.abs().max(1.) * 1e-5,
        "{actual} != {expected}"
    );
}

#[test]
fn imported_box_transform_cut_and_step_roundtrip_use_verified_source_geometry() {
    let directory = tempfile::tempdir().unwrap();
    let (bytes, hash) = source(directory.path());
    stage(directory.path(), &hash, &bytes);
    let model = document(&hash);
    assert_eq!(
        build_body(&model, "body").err().unwrap().code,
        "ASSET_MISSING"
    );
    let cut = build_body_with_assets(&model, "body", Some(directory.path())).unwrap();
    close(cut.volume_mm3(), 8000. - 40. * std::f64::consts::PI);
    let path = directory.path().join("modified.step");
    cut.write_step(&path).unwrap();
    let reopened = Solid::read_step(&path).unwrap();
    close(reopened.volume_mm3(), cut.volume_mm3());
    close(reopened.surface_area_mm2(), cut.surface_area_mm2());
    let mut moved = model.clone();
    moved.bodies[0].source_feature_id = "moved".into();
    let moved = build_body_with_assets(&moved, "body", Some(directory.path())).unwrap();
    let target = Solid::box_solid(40., 20., 10.)
        .unwrap()
        .translated(10., 0., 0.)
        .unwrap();
    close(moved.intersect(&target).unwrap().volume_mm3(), 8000.);
    assert_eq!(
        std::fs::read(directory.path().join("original.step")).unwrap(),
        bytes
    );
}

#[test]
fn imported_box_fillet_and_unresolved_selected_edge_do_not_guess() {
    let directory = tempfile::tempdir().unwrap();
    let (bytes, hash) = source(directory.path());
    stage(directory.path(), &hash, &bytes);
    let mut value = serde_json::to_value(document(&hash)).unwrap();
    value["features"].as_array_mut().unwrap().push(json!({"id":"rounded","name":"Rounded","operation":{"type":"fillet","bodyFeatureId":"imported","radius":{"kind":"literal","mm":1}}}));
    value["bodies"][0]["sourceFeatureId"] = "rounded".into();
    let rounded: Document = serde_json::from_value(value.clone()).unwrap();
    let volume = build_body_with_assets(&rounded, "body", Some(directory.path()))
        .unwrap()
        .volume_mm3();
    assert!(volume > 0. && volume < 8000.);
    value["features"]
        .as_array_mut()
        .unwrap()
        .last_mut()
        .unwrap()["operation"] = json!({"type":"filletEdge","bodyFeatureId":"imported","edgeKey":"box-edge:x:ymin:zmiddle","radius":{"kind":"literal","mm":1}});
    assert!(serde_json::from_value::<Document>(value)
        .unwrap()
        .validate()
        .is_err());
    // The selector is valid for an axis-aligned box, but no longer resolves after an oblique rotation.
    let mut value = serde_json::to_value(document(&hash)).unwrap();
    value["features"].as_array_mut().unwrap().extend([
        json!({"id":"oblique","name":"Oblique","operation":{"type":"rotate","bodyFeatureId":"imported","axisOriginMm":[0,0,0],"axisDirection":[0,0,1],"angleDeg":17}}),
        json!({"id":"edge","name":"Edge","operation":{"type":"filletEdge","bodyFeatureId":"oblique","edgeKey":"box-edge:x:ymin:zmin","radius":{"kind":"literal","mm":1}}})]);
    value["bodies"][0]["sourceFeatureId"] = "edge".into();
    assert!(build_body_with_assets(
        &serde_json::from_value(value).unwrap(),
        "body",
        Some(directory.path())
    )
    .is_err());
}

#[tokio::test]
async fn worker_import_pipeline_and_missing_tampered_inputs_preserve_last_saved_artifact() {
    let source_directory = tempfile::tempdir().unwrap();
    let (bytes, hash) = source(source_directory.path());
    let saved = source_directory.path().join("last-committed.step");
    std::fs::write(&saved, &bytes).unwrap();
    let worker = Path::new(env!("CARGO_BIN_EXE_forma-cad-worker"));
    for state in ["valid", "missing", "tampered"] {
        let directory = tempfile::tempdir().unwrap();
        if state != "missing" {
            stage(
                directory.path(),
                &hash,
                if state == "valid" {
                    &bytes
                } else {
                    b"tampered STEP"
                },
            );
        }
        let result = forma_core::native::worker::build_with_executable(
            &serde_json::to_string(&document(&hash)).unwrap(),
            directory.path(),
            Arc::new(AtomicBool::new(false)),
            worker,
        )
        .await;
        if state == "valid" {
            result.unwrap();
            close(
                Solid::read_step(&directory.path().join("model.step"))
                    .unwrap()
                    .volume_mm3(),
                8000. - 40. * std::f64::consts::PI,
            );
            assert!(directory.path().join("preview.glb").exists());
        } else {
            let error = result.unwrap_err().to_string();
            assert!(
                error.contains(if state == "missing" {
                    "ASSET_MISSING"
                } else {
                    "ASSET_CHECKSUM_MISMATCH"
                }),
                "{error}"
            );
            assert!(!directory.path().join("model.step").exists());
            assert!(!directory.path().join("preview.glb").exists());
        }
        assert_eq!(std::fs::read(&saved).unwrap(), bytes);
    }
}

#[test]
fn paths_unknown_fields_hash_changes_and_failed_command_batches_cannot_replace_import() {
    let hash = "a".repeat(64);
    let original = document(&hash);
    for operation in [
        json!({"type":"importStep","assetId":"../../secret","sha256":hash}),
        json!({"type":"importStep","assetId":format!("step_{hash}"),"sha256":hash,"path":"C:/secret.step"}),
    ] {
        let mut value = serde_json::to_value(&original).unwrap();
        value["features"][0]["operation"] = operation;
        if let Ok(document) = serde_json::from_value::<Document>(value) {
            assert!(document.validate().is_err());
        }
    }
    let commands:Vec<Command>=serde_json::from_value(json!([
        {"command":"set_body_source","bodyId":"body","sourceFeatureId":"imported"},
        {"command":"add_feature","feature":{"id":"bad","name":"Bad","operation":{"type":"importStep","assetId":format!("step_{hash}"),"sha256":"b".repeat(64)}}}
    ])).unwrap();
    assert_eq!(
        apply_commands(&original, "import_base", "rejected", &commands)
            .unwrap_err()
            .code,
        ErrorCode::InvalidValue
    );
    assert_eq!(original.bodies[0].source_feature_id, "cut");
}

#[tokio::test]
async fn host_staging_is_project_scoped_content_addressed_and_preserves_metadata() {
    let directory = tempfile::tempdir().unwrap();
    let (_, guard) = tracing_appender::non_blocking(std::io::sink());
    let state = forma_core::core::AppState {
        cad_tasks: Default::default(),
        project_access: Default::default(),
        root: directory.path().to_path_buf(),
        pool: forma_core::storage::open(&directory.path().join("fixture.sqlite"))
            .await
            .unwrap(),
        grants: tokio::sync::Mutex::new(Default::default()),
        tasks: tokio::sync::Mutex::new(Default::default()),
        writes: tokio::sync::Mutex::new(()),
        _log_guard: guard,
    };
    let (bytes, hash) = source(directory.path());
    let file = json!({"name":"input.STP","kind":"attachment","size":bytes.len(),"sha256":hash,"data":forma_core::artifacts::data_url("input.STP",&bytes)});
    let mut project:forma_core::models::Project=serde_json::from_value(json!({
        "schemaVersion":1,"id":uuid::Uuid::new_v4().to_string(),"name":"Import fixture","units":"mm","agent":"codex",
        "pinned":false,"createdAt":"2026-10-04T00:00:00Z","updatedAt":"2026-10-04T00:00:00Z","revisions":[],
        "currentRevision":null,"messages":[],"files":[file.clone(),file],"exports":[]
    })).unwrap();
    let snapshot = serde_json::to_string(&project).unwrap();
    let job = tempfile::tempdir().unwrap();
    forma_core::native::assets::stage_project_assets(
        &state,
        &project,
        &document(&hash),
        job.path(),
    )
    .unwrap();
    assert_eq!(
        std::fs::read(
            job.path()
                .join(forma_core::native::assets::relative_path(&hash))
        )
        .unwrap(),
        bytes
    );
    assert_eq!(
        std::fs::read_dir(job.path().join("inputs"))
            .unwrap()
            .count(),
        1
    );
    assert_eq!(snapshot, serde_json::to_string(&project).unwrap());
    project.files.clear();
    let rejected = tempfile::tempdir().unwrap();
    assert!(forma_core::native::assets::stage_project_assets(
        &state,
        &project,
        &document(&hash),
        rejected.path()
    )
    .unwrap_err()
    .to_string()
    .contains("ASSET_MISSING"));
    assert!(!rejected.path().join("inputs").exists());
}

#[test]
fn input_count_size_and_unused_asset_checks_happen_before_artifacts() {
    let mut value = serde_json::to_value(document(&"a".repeat(64))).unwrap();
    for i in 0..32 {
        let hash = format!("{i:064x}");
        value["features"].as_array_mut().unwrap().push(json!({"id":format!("asset{i}"),"name":"Asset","operation":{"type":"importStep","assetId":format!("step_{hash}"),"sha256":hash}}));
    }
    assert_eq!(
        serde_json::from_value::<Document>(value)
            .unwrap()
            .validate()
            .unwrap_err()
            .code,
        ErrorCode::InvalidDocument
    );
    let directory = tempfile::tempdir().unwrap();
    let (bytes, hash) = source(directory.path());
    stage(directory.path(), &hash, &bytes);
    let mut model = document(&hash);
    model.bodies[0].source_feature_id = "tool".into();
    std::fs::remove_file(
        directory
            .path()
            .join(forma_core::native::assets::relative_path(&hash)),
    )
    .unwrap();
    assert_eq!(
        forma_core::native::job::execute(&model, None, directory.path())
            .err()
            .unwrap()
            .code,
        "ASSET_MISSING"
    );
    assert!(!directory.path().join("model.step").exists());
    let path = directory
        .path()
        .join(forma_core::native::assets::relative_path(&hash));
    std::fs::File::create(&path)
        .unwrap()
        .set_len(forma_core::artifacts::MAX_FILE_BYTES as u64 + 1)
        .unwrap();
    assert_eq!(
        forma_core::native::assets::read_step(directory.path(), &format!("step_{hash}"), &hash)
            .err()
            .unwrap()
            .code,
        "ASSET_LIMIT"
    );
}
