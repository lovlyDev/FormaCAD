#![cfg(feature = "native-occt")]

use forma_core::{
    cad_ir::{apply_commands, Command, Document, ErrorCode},
    native::{build_body, Solid},
};

fn document() -> Document {
    serde_json::from_value(serde_json::json!({
        "schemaVersion":2,"revisionId":"transform_base","parameters":[],
        "features":[
            {"id":"profile","name":"Profile","operation":{"type":"rectangle","width":{"kind":"literal","mm":40.0},"depth":{"kind":"literal","mm":20.0}}},
            {"id":"pad","name":"Pad","operation":{"type":"extrude","sketchId":"profile","distance":{"kind":"literal","mm":10.0}}},
            {"id":"rotation","name":"Rotation","operation":{"type":"rotate","bodyFeatureId":"pad","axisOriginMm":[3.0,4.0,5.0],"axisDirection":[1.0,2.0,3.0],"angleDeg":73.0}},
            {"id":"mirror","name":"Mirror","operation":{"type":"mirror","bodyFeatureId":"rotation","planeOriginMm":[7.0,11.0,13.0],"planeNormal":[1.0,-2.0,3.0]}}
        ],"bodies":[{"id":"body","name":"Body","sourceFeatureId":"mirror"}]
    })).unwrap()
}

fn close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-5 * expected.abs().max(1.0),
        "{actual} != {expected}"
    );
}

fn round_trip(solid: &Solid) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("transform.step");
    solid.write_step(&path).unwrap();
    let reopened = Solid::read_step(&path).unwrap();
    close(reopened.volume_mm3(), solid.volume_mm3());
    close(reopened.surface_area_mm2(), solid.surface_area_mm2());
    for (actual, expected) in reopened.bounds_mm().into_iter().zip(solid.bounds_mm()) {
        close(actual, expected);
    }
    assert_eq!(reopened.face_count(), solid.face_count());
    assert_eq!(reopened.edge_count(), solid.edge_count());
}

#[test]
fn arbitrary_axis_and_oblique_plane_preserve_brep_invariants_and_step() {
    let original = document();
    let solid = build_body(&original, "body").unwrap();
    close(solid.volume_mm3(), 8000.0);
    close(solid.surface_area_mm2(), 2800.0);
    assert_eq!(solid.face_count(), 6);
    assert_eq!(solid.edge_count(), 12);
    round_trip(&solid);
    assert_eq!(original.revision_id, "transform_base");

    // Independent Rodrigues rotation of all eight corners gives the exact AABB.
    let mut rotation = original.clone();
    rotation.bodies[0].source_feature_id = "rotation".into();
    let rotated = build_body(&rotation, "body").unwrap();
    let axis = [1.0 / 14f64.sqrt(), 2.0 / 14f64.sqrt(), 3.0 / 14f64.sqrt()];
    let angle = 73f64.to_radians();
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for x in [-20.0, 20.0] {
        for y in [-10.0, 10.0] {
            for z in [0.0, 10.0] {
                let v = [x - 3.0, y - 4.0, z - 5.0];
                let dot = axis.iter().zip(v).map(|(a, b)| a * b).sum::<f64>();
                let cross = [
                    axis[1] * v[2] - axis[2] * v[1],
                    axis[2] * v[0] - axis[0] * v[2],
                    axis[0] * v[1] - axis[1] * v[0],
                ];
                for i in 0..3 {
                    let point = v[i] * angle.cos()
                        + cross[i] * angle.sin()
                        + axis[i] * dot * (1.0 - angle.cos())
                        + [3.0, 4.0, 5.0][i];
                    min[i] = min[i].min(point);
                    max[i] = max[i].max(point);
                }
            }
        }
    }
    for i in 0..3 {
        close(rotated.bounds_mm()[i], max[i] - min[i]);
    }
}

#[test]
fn axis_origin_is_applied_and_mirror_changes_position_not_only_normals() {
    let source = Solid::box_solid(10.0, 4.0, 2.0)
        .unwrap()
        .translated(20.0, 0.0, 0.0)
        .unwrap();
    let rotated = source
        .rotated([10.0, 0.0, 0.0], [0.0, 0.0, 7.0], 180.0)
        .unwrap();
    let target = Solid::box_solid(10.0, 4.0, 2.0).unwrap();
    close(rotated.intersect(&target).unwrap().volume_mm3(), 80.0);
    let mirrored = source.mirrored([10.0, 0.0, 0.0], [9.0, 0.0, 0.0]).unwrap();
    close(mirrored.intersect(&target).unwrap().volume_mm3(), 80.0);
    close(source.volume_mm3(), 80.0);
    round_trip(&mirrored);
}

#[test]
fn modifier_commands_are_atomic_preserve_ids_and_suppression_passes_through() {
    let current = document();
    let commands: Vec<Command> = serde_json::from_value(serde_json::json!([
        {"command":"set_rotation","featureId":"rotation","axisOriginMm":[0,0,0],"axisDirection":[0,0,1],"angleDeg":90},
        {"command":"set_mirror_plane","featureId":"mirror","planeOriginMm":[0,0,0],"planeNormal":[1,0,0]}
    ])).unwrap();
    let changed =
        apply_commands(&current, "transform_base", "transform_changed", &commands).unwrap();
    close(build_body(&changed, "body").unwrap().bounds_mm()[0], 20.0);
    close(build_body(&changed, "body").unwrap().bounds_mm()[1], 40.0);
    assert_eq!(current, document());
    assert_eq!(changed.features[2].id, "rotation");
    let mut suppressed = changed.clone();
    suppressed.features[2].suppressed = true;
    suppressed.features[3].suppressed = true;
    assert_eq!(
        build_body(&suppressed, "body").unwrap().bounds_mm(),
        [40.0, 20.0, 10.0]
    );

    let mut invalid = commands;
    invalid.push(serde_json::from_value(serde_json::json!({"command":"set_mirror_plane","featureId":"mirror","planeOriginMm":[0,0,0],"planeNormal":[0,0,0]})).unwrap());
    assert_eq!(
        apply_commands(&current, "transform_base", "failed", &invalid)
            .unwrap_err()
            .code,
        ErrorCode::InvalidValue
    );
    assert_eq!(current, document());
    let wrong: Command = serde_json::from_value(serde_json::json!({"command":"set_rotation","featureId":"pad","axisOriginMm":[0,0,0],"axisDirection":[0,0,1],"angleDeg":30})).unwrap();
    assert_eq!(
        apply_commands(&current, "transform_base", "failed", &[wrong])
            .unwrap_err()
            .code,
        ErrorCode::UnsupportedField
    );
}

#[test]
fn degenerate_axes_angles_and_reference_types_fail_at_both_boundaries() {
    let source = Solid::box_solid(40.0, 20.0, 10.0).unwrap();
    for vector in [
        [0.0, 0.0, 0.0],
        [1e-10, 0.0, 0.0],
        [f64::NAN, 0.0, 1.0],
        [1e7, 0.0, 0.0],
    ] {
        assert_eq!(
            source.rotated([0.0; 3], vector, 90.0).err().unwrap().code,
            "INVALID_TRANSFORM"
        );
        assert_eq!(
            source.mirrored([0.0; 3], vector).err().unwrap().code,
            "INVALID_TRANSFORM"
        );
    }
    for angle in [361.0, -361.0, f64::INFINITY, f64::NAN] {
        assert_eq!(
            source
                .rotated([0.0; 3], [0.0, 0.0, 1.0], angle)
                .err()
                .unwrap()
                .code,
            "INVALID_TRANSFORM"
        );
    }
    close(
        source
            .rotated([0.0; 3], [0.0, 0.0, 1.0], 0.0)
            .unwrap()
            .volume_mm3(),
        8000.0,
    );
    close(
        source
            .rotated([0.0; 3], [0.0, 0.0, 1.0], -360.0)
            .unwrap()
            .volume_mm3(),
        8000.0,
    );
    let mut invalid = serde_json::to_value(document()).unwrap();
    invalid["features"][2]["operation"]["bodyFeatureId"] = "profile".into();
    assert_eq!(
        serde_json::from_value::<Document>(invalid)
            .unwrap()
            .validate()
            .unwrap_err()
            .code,
        ErrorCode::ReferenceTypeMismatch
    );
}

#[test]
fn legacy_sixteen_tooth_cog_matches_independent_circle_rectangle_measurement() {
    let mut features = vec![
        serde_json::json!({"id":"disc_profile","name":"Disc","operation":{"type":"circle","radius":{"kind":"literal","mm":20}}}),
        serde_json::json!({"id":"disc","name":"Disc pad","operation":{"type":"extrude","sketchId":"disc_profile","distance":{"kind":"literal","mm":8}}}),
        serde_json::json!({"id":"tooth_profile","name":"Tooth","operation":{"type":"rectangle","width":{"kind":"literal","mm":7},"depth":{"kind":"literal","mm":3}}}),
        serde_json::json!({"id":"tooth_pad","name":"Tooth pad","operation":{"type":"extrude","sketchId":"tooth_profile","distance":{"kind":"literal","mm":8}}}),
        serde_json::json!({"id":"tooth","name":"Radial tooth","operation":{"type":"translate","bodyFeatureId":"tooth_pad","offsetMm":[22,0,0]}}),
    ];
    let mut previous = "disc".to_owned();
    for i in 0..16 {
        let rotated = format!("tooth_{i}");
        let output = format!("fuse_{i}");
        features.push(serde_json::json!({"id":rotated,"name":"Rotated tooth","operation":{"type":"rotate","bodyFeatureId":"tooth","axisOriginMm":[0,0,0],"axisDirection":[0,0,1],"angleDeg":i as f64*22.5}}));
        features.push(serde_json::json!({"id":output,"name":"Tooth union","operation":{"type":"boolean","leftFeatureId":previous,"rightFeatureId":rotated,"mode":"union"}}));
        previous = output;
    }
    features.push(serde_json::json!({"id":"hole","name":"Center hole","operation":{"type":"hole","bodyFeatureId":previous,"centerMm":[0,0],"startZMm":0,"radius":{"kind":"literal","mm":4},"depth":{"kind":"literal","mm":8}}}));
    let model: Document = serde_json::from_value(serde_json::json!({"schemaVersion":2,"revisionId":"cog","parameters":[],"features":features,"bodies":[{"id":"body","name":"Cog","sourceFeatureId":"hole"}]})).unwrap();
    let cog = build_body(&model, "body").unwrap();
    let outside_tooth_area =
        25.5 * 3.0 - (1.5 * (400.0_f64 - 2.25).sqrt() + 400.0 * (1.5_f64 / 20.0).asin());
    let expected = (std::f64::consts::PI * (400.0 - 16.0) + 16.0 * outside_tooth_area) * 8.0;
    close(cog.volume_mm3(), expected);
    close(cog.bounds_mm()[0], 51.0);
    close(cog.bounds_mm()[1], 51.0);
    close(cog.bounds_mm()[2], 8.0);
    round_trip(&cog);
}

#[tokio::test]
async fn isolated_worker_emits_transformed_step_glb_and_rejects_degenerate_axis_without_artifacts()
{
    let directory = tempfile::tempdir().unwrap();
    let worker = std::path::Path::new(env!("CARGO_BIN_EXE_forma-cad-worker"));
    let model = document();
    forma_core::native::worker::build_with_executable(
        &serde_json::to_string(&model).unwrap(),
        directory.path(),
        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        worker,
    )
    .await
    .unwrap();
    assert!(
        directory
            .path()
            .join("preview.glb")
            .metadata()
            .unwrap()
            .len()
            > 100
    );
    let reopened = Solid::read_step(&directory.path().join("model.step")).unwrap();
    close(reopened.volume_mm3(), 8000.0);
    let mut invalid = serde_json::to_value(model).unwrap();
    invalid["features"][3]["operation"]["planeNormal"] = serde_json::json!([0, 0, 0]);
    let rejected = tempfile::tempdir().unwrap();
    assert!(forma_core::native::worker::build_with_executable(
        &invalid.to_string(),
        rejected.path(),
        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        worker
    )
    .await
    .is_err());
    assert!(!rejected.path().join("model.step").exists());
    assert!(!rejected.path().join("preview.glb").exists());
}
