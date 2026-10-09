#![cfg(feature = "native-occt")]
//! Genuine isolated worker sections of sealed whole committed geometry.
#[path = "face_sections/failures.rs"]
mod failures;
#[path = "face_sections/fixture.rs"]
mod fixture;
use fixture::*;
#[tokio::test]
async fn selected_box_face_intersects_every_committed_body_and_preserves_sources() {
    let (directory, binding) = staged();
    let source = std::fs::read(directory.path().join("source.step")).unwrap();
    let raw = std::fs::read(directory.path().join("document.json")).unwrap();
    // Separate owned stage directories because report publication is immutable.
    for role in ["box-face:zmax", "box-face:zmin"] {
        let case = tempfile::tempdir().unwrap();
        std::fs::write(case.path().join("source.step"), &source).unwrap();
        std::fs::write(case.path().join("document.json"), &raw).unwrap();
        let report = section(case.path(), &binding, &query(role, -5.)).await;
        assert!(
            (report.geometry.total_length_mm - (120. + 16. * std::f64::consts::PI)).abs() < 1e-6
        );
        assert!((report.geometry.plane.origin_mm[2] - 5.).abs() < 1e-8);
        assert_eq!(
            report.geometry.plane.normal[2],
            if role.ends_with("zmax") { 1. } else { -1. }
        );
        assert!(report
            .geometry
            .curves
            .iter()
            .flat_map(|c| &c.points_mm)
            .any(|p| p[0] > 90.));
        assert_eq!(
            std::fs::read(case.path().join("source.step")).unwrap(),
            source
        );
        assert_eq!(
            std::fs::read(case.path().join("document.json")).unwrap(),
            raw
        );
        assert!(!case.path().join("model.step").exists());
        assert!(!case.path().join("preview.glb").exists());
        assert!(!case.path().join("measurement.json").exists());
    }
}
#[tokio::test]
async fn positive_offset_above_all_bodies_is_verified_empty_not_a_failure() {
    let (directory, binding) = staged();
    let report = section(directory.path(), &binding, &query("box-face:zmax", 6.)).await;
    for (actual, expected) in report
        .geometry
        .plane
        .origin_mm
        .into_iter()
        .zip([0., 0., 16.])
    {
        assert!((actual - expected).abs() < 1e-8);
    }
    assert!(report.geometry.curves.is_empty());
    assert_eq!(report.geometry.total_length_mm, 0.);
}
