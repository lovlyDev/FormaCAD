#![cfg(feature = "native-occt")]

use forma_core::{
    cad_ir::{apply_commands, Command, Document, ErrorCode},
    native::{build_body, Solid},
};
use serde_json::{json, Value};
use std::f64::consts::PI;

fn model(plane: &str, holes: bool, angle: f64) -> Document {
    let mut points = vec![];
    let mut lines = vec![];
    let loops = if holes { 2 } else { 1 };
    for (loop_id, coordinates) in [
        vec![[2., 0.], [6., 0.], [6., 10.], [2., 10.]],
        vec![[3., 2.], [4., 2.], [4., 5.], [3., 5.]],
    ]
    .into_iter()
    .take(loops)
    .enumerate()
    {
        for (i, [x, y]) in coordinates.into_iter().enumerate() {
            let id = format!("p{loop_id}_{i}");
            points.push(json!({"id":id,"xMm":x,"yMm":y}));
            lines.push(json!({"id":format!("l{loop_id}_{i}"),"startPointId":id,"endPointId":format!("p{loop_id}_{}",(i+1)%4)}));
        }
    }
    let axis = if plane == "xy" {
        [0., 1., 0.]
    } else {
        [0., 0., 1.]
    };
    serde_json::from_value(json!({"schemaVersion":2,"revisionId":"revolution_base","parameters":[],"features":[
        {"id":"profile","name":"Profile","operation":{"type":"sketch2d","plane":plane,"originMm":[3,4,5],"points":points,"lines":lines,"constraints":[]}},
        {"id":"revolution","name":"Revolution","operation":{"type":"revolve","sketchId":"profile","axisOriginMm":[3,4,5],"axisDirection":axis,"angleDeg":angle}}
    ],"bodies":[{"id":"body","name":"Body","sourceFeatureId":"revolution"}]})).unwrap()
}

fn close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-5 * expected.abs().max(1.),
        "{actual} != {expected}"
    );
}

#[test]
fn full_revolution_holes_planes_signs_and_step_match_analytic_ring() {
    for plane in ["xy", "xz", "yz"] {
        for angle in [-360., 360.] {
            for holes in [false, true] {
                let solid = build_body(&model(plane, holes, angle), "body").unwrap();
                close(
                    solid.volume_mm3(),
                    if holes { 299. * PI } else { 320. * PI },
                );
                close(
                    solid.surface_area_mm2(),
                    if holes { 280. * PI } else { 224. * PI },
                );
                let expected = if plane == "xy" {
                    [12., 10., 12.]
                } else {
                    [12., 12., 10.]
                };
                for (a, b) in solid.bounds_mm().into_iter().zip(expected) {
                    close(a, b);
                }
                let directory = tempfile::tempdir().unwrap();
                let path = directory.path().join("revolution.step");
                solid.write_step(&path).unwrap();
                let reopened = Solid::read_step(&path).unwrap();
                close(reopened.volume_mm3(), solid.volume_mm3());
                close(reopened.surface_area_mm2(), solid.surface_area_mm2());
            }
        }
    }
}

#[test]
fn partial_revolution_and_typed_batch_preserve_source_and_reject_invalid_last_command() {
    let original = model("xz", false, 360.);
    let first = Command::SetRevolution {
        feature_id: "revolution".into(),
        axis_origin_mm: [3., 4., 5.],
        axis_direction: [0., 0., 7.],
        angle_deg: 90.,
    };
    let staged = apply_commands(
        &original,
        "revolution_base",
        "quarter",
        std::slice::from_ref(&first),
    )
    .unwrap();
    close(build_body(&staged, "body").unwrap().volume_mm3(), 80. * PI);
    close(
        build_body(&staged, "body").unwrap().surface_area_mm2(),
        56. * PI + 80.,
    );
    let invalid = Command::SetRevolution {
        feature_id: "revolution".into(),
        axis_origin_mm: [3., 4., 5.],
        axis_direction: [0.; 3],
        angle_deg: 180.,
    };
    assert_eq!(
        apply_commands(&original, "revolution_base", "rejected", &[first, invalid])
            .unwrap_err()
            .code,
        ErrorCode::InvalidValue
    );
    close(
        build_body(&original, "body").unwrap().volume_mm3(),
        320. * PI,
    );
    assert_eq!(original.revision_id, "revolution_base");
}

#[test]
fn off_plane_crossing_axis_zero_angle_and_wrong_profile_are_rejected() {
    let original = serde_json::to_value(model("xz", false, 360.)).unwrap();
    for (field, value) in [
        ("axisOriginMm", json!([3, 5, 5])),
        ("axisDirection", json!([0, 1, 0])),
        ("axisOriginMm", json!([7, 4, 5])),
        ("angleDeg", json!(0)),
        ("angleDeg", json!(361)),
    ] {
        let mut invalid = original.clone();
        invalid["features"][1]["operation"][field] = value;
        let document: Document = serde_json::from_value(invalid).unwrap();
        assert!(build_body(&document, "body").is_err(), "accepted {field}");
    }
    let mut wrong = original;
    wrong["features"][0]["operation"] = json!({"type":"rectangle","width":{"kind":"literal","mm":4},"depth":{"kind":"literal","mm":10}});
    assert_eq!(
        serde_json::from_value::<Document>(wrong)
            .unwrap()
            .validate()
            .unwrap_err()
            .code,
        ErrorCode::ReferenceTypeMismatch
    );
}

#[tokio::test]
async fn isolated_worker_revolution_exports_and_failed_geometry_leaves_no_artifacts() {
    let worker = std::path::Path::new(env!("CARGO_BIN_EXE_forma-cad-worker"));
    for valid in [true, false] {
        let mut document: Value = serde_json::to_value(model("xz", true, 180.)).unwrap();
        if !valid {
            document["features"][1]["operation"]["axisOriginMm"] = json!([7, 4, 5]);
        }
        let directory = tempfile::tempdir().unwrap();
        let result = forma_core::native::worker::build_with_executable(
            &document.to_string(),
            directory.path(),
            std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            worker,
        )
        .await;
        if valid {
            result.unwrap();
            close(
                Solid::read_step(&directory.path().join("model.step"))
                    .unwrap()
                    .volume_mm3(),
                149.5 * PI,
            );
            assert!(
                directory
                    .path()
                    .join("preview.glb")
                    .metadata()
                    .unwrap()
                    .len()
                    > 100
            );
        } else {
            assert!(result.is_err());
            assert!(!directory.path().join("model.step").exists());
            assert!(!directory.path().join("preview.glb").exists());
        }
    }
}
