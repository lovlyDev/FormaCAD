//! Direct CXX geometry/STEP round-trip and explicit owned temporary cleanup.
use super::*;

#[test]
fn cxx_bridge_builds_valid_brep() {
    let directory = tempfile::tempdir().unwrap();
    let directory_path = directory.path().to_path_buf();
    let path = directory.path().join("round-trip.step");
    {
        assert!((box_volume(60.0, 40.0, 10.0).unwrap() - 24000.0).abs() < 0.001);
        assert_eq!(
            box_volume(0.0, 40.0, 10.0).unwrap_err().code,
            "INVALID_DIMENSION"
        );
        let stock = Solid::box_solid(60.0, 40.0, 10.0).unwrap();
        assert!((stock.surface_area_mm2() - 6800.0).abs() < 0.001);
        assert_eq!((stock.face_count(), stock.edge_count()), (6, 12));
        let tool = Solid::cylinder(5.0, 10.0).unwrap();
        let cut = stock.cut(&tool).unwrap();
        let expected = 24000.0 - 250.0 * std::f64::consts::PI;
        assert!((cut.volume_mm3() - expected).abs() < 0.01);
        assert!((cut.translated(5.0, 0.0, 0.0).unwrap().volume_mm3() - expected).abs() < 0.01);
        cut.write_step(&path).unwrap();
        let imported = Solid::read_step(&path).unwrap();
        assert!((imported.volume_mm3() - cut.volume_mm3()).abs() < 0.01);
        assert!(path.is_file());
    }
    // Drop every native shape before checking cleanup. close surfaces failures;
    // unlike TempDir's destructor, it does not silently ignore cleanup errors.
    directory
        .close()
        .expect("owned STEP directory must close after native shapes drop");
    assert!(!path.exists());
    assert!(!directory_path.exists());
}
