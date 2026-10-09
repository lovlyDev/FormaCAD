#![cfg(feature = "native-occt")]
//! Genuine isolated worker pair values with analytic oracles and sealed inputs.
#[path = "pair_measurements/failures.rs"]
mod failures;
#[path = "pair_measurements/fixture.rs"]
mod fixture;
#[path = "pair_measurements/transforms.rs"]
mod transforms;
use fixture::*;
use forma_core::model_pair_measurement::schema::PairMeasurementValue;
#[tokio::test]
async fn genuine_worker_distances_and_angle_conventions_accept_zero_and_preserve_saved_bytes() {
    for (kind, a, b, expected) in [
        ("minimumDistance", "box-face:xmin", "box-face:xmax", 40.),
        ("minimumDistance", "box-face:xmin", "box-face:xmin", 0.),
        (
            "minimumDistance",
            "box-edge:x:ymin:zmin",
            "box-edge:x:ymax:zmax",
            500_f64.sqrt(),
        ),
        (
            "minimumDistance",
            "box-face:xmin",
            "box-edge:y:xmax:zmin",
            40.,
        ),
        ("faceNormalAngle", "box-face:xmin", "box-face:xmax", 180.),
        ("faceNormalAngle", "box-face:xmin", "box-face:ymin", 90.),
        ("faceNormalAngle", "box-face:xmin", "box-face:xmin", 0.),
        (
            "edgeAcuteAngle",
            "box-edge:x:ymin:zmin",
            "box-edge:x:ymax:zmax",
            0.,
        ),
        (
            "edgeAcuteAngle",
            "box-edge:x:ymin:zmin",
            "box-edge:y:xmin:zmin",
            90.,
        ),
    ] {
        let (dir, binding) = staged();
        let source = std::fs::read(dir.path().join("source.step")).unwrap();
        let raw = std::fs::read(dir.path().join("document.json")).unwrap();
        let q = query(kind, a, b);
        let report = measure(dir.path(), &binding, &q).await.unwrap();
        assert!(report.result.valid_for(&q));
        let actual = match report.result {
            PairMeasurementValue::MinimumDistance { distance_mm, .. } => distance_mm,
            PairMeasurementValue::FaceNormalAngle { angle_deg }
            | PairMeasurementValue::EdgeAcuteAngle { angle_deg } => angle_deg,
        };
        assert!(
            (actual - expected).abs() < 1e-6,
            "{kind}: {actual} != {expected}"
        );
        assert_eq!(
            std::fs::read(dir.path().join("source.step")).unwrap(),
            source
        );
        assert_eq!(
            std::fs::read(dir.path().join("document.json")).unwrap(),
            raw
        );
        assert!(!dir.path().join("model.step").exists());
        assert!(!dir.path().join("preview.glb").exists());
    }
}
