#![cfg(feature = "native-occt")]
use forma_core::native::{
    section::{section_with_executable, SectionPlane},
    Solid,
};
use std::{
    path::Path,
    sync::{atomic::AtomicBool, Arc},
};
fn plane(origin: [f64; 3], normal: [f64; 3]) -> SectionPlane {
    SectionPlane {
        origin_mm: origin,
        normal,
        deflection_mm: 0.01,
    }
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < b.abs().max(1.) * 1e-6, "{a} != {b}");
}
fn stage(cwd: &Path, solid: &Solid) -> String {
    let path = cwd.join("original.step");
    solid.write_step(&path).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let hash = forma_core::artifacts::digest(&bytes);
    forma_core::artifacts::immutable_write(
        cwd,
        &forma_core::native::assets::relative_path(&hash),
        &bytes,
    )
    .unwrap();
    hash
}

#[test]
fn exact_box_sections_in_three_planes_and_free_plane_preserve_source() {
    let source = Solid::box_solid(40., 20., 10.).unwrap();
    for (plane, expected) in [
        (plane([0., 0., 5.], [0., 0., 7.]), 120.),
        (plane([0., 0., 5.], [0., 3., 0.]), 100.),
        (plane([0., 0., 5.], [2., 0., 0.]), 60.),
        (plane([0., 0., 5.], [1., 1., 0.]), 40. * 2f64.sqrt() + 20.),
    ] {
        let result = source.section(&plane).unwrap();
        close(result.total_length_mm, expected);
        assert_eq!(result.curves.len(), 4);
        for curve in result.curves {
            assert_eq!(curve.points_mm.len(), 2);
            for point in curve.points_mm {
                let distance = point
                    .into_iter()
                    .zip(plane.origin_mm)
                    .zip(plane.normal)
                    .map(|((p, o), n)| (p - o) * n)
                    .sum::<f64>();
                assert!(distance.abs() < 1e-5);
            }
        }
        close(source.volume_mm3(), 8000.);
        close(source.surface_area_mm2(), 2800.);
    }
}

#[test]
fn hollow_cylinder_has_exact_two_circle_lengths_independent_of_display_sampling() {
    let source = Solid::cylinder(10., 20.)
        .unwrap()
        .cut(&Solid::cylinder(4., 20.).unwrap())
        .unwrap();
    for deflection in [0.001, 0.01, 1.] {
        let mut plane = plane([0., 0., 7.], [0., 0., 1.]);
        plane.deflection_mm = deflection;
        let result = source.section(&plane).unwrap();
        assert_eq!(result.curves.len(), 2);
        close(result.total_length_mm, 28. * std::f64::consts::PI);
        let mut lengths = result
            .curves
            .iter()
            .map(|c| c.length_mm)
            .collect::<Vec<_>>();
        lengths.sort_by(f64::total_cmp);
        close(lengths[0], 8. * std::f64::consts::PI);
        close(lengths[1], 20. * std::f64::consts::PI);
        for curve in result.curves {
            assert!(curve.closed);
            assert!(curve.points_mm.len() > 4);
            let polyline = curve
                .points_mm
                .windows(2)
                .map(|pair| {
                    pair[0]
                        .into_iter()
                        .zip(pair[1])
                        .map(|(a, b)| (a - b).powi(2))
                        .sum::<f64>()
                        .sqrt()
                })
                .sum::<f64>();
            assert!(polyline < curve.length_mm);
            for p in curve.points_mm {
                close(p[2], 7.);
            }
        }
    }
}

#[test]
fn empty_sections_are_valid_and_degenerate_planes_fail_before_geometry() {
    let source = Solid::box_solid(40., 20., 10.).unwrap();
    let empty = source.section(&plane([0., 0., 50.], [0., 0., 1.])).unwrap();
    assert!(empty.curves.is_empty());
    assert_eq!(empty.total_length_mm, 0.);
    for invalid in [
        plane([0.; 3], [0.; 3]),
        plane([f64::NAN, 0., 0.], [0., 0., 1.]),
        plane([0.; 3], [1e7, 0., 0.]),
        SectionPlane {
            origin_mm: [0.; 3],
            normal: [0., 0., 1.],
            deflection_mm: 0.,
        },
    ] {
        assert_eq!(
            source.section(&invalid).unwrap_err().code,
            "INVALID_SECTION_PLANE"
        );
    }
}

#[tokio::test]
async fn isolated_section_worker_is_read_only_checksum_bound_and_cancellable() {
    let worker = Path::new(env!("CARGO_BIN_EXE_forma-cad-worker"));
    for scenario in ["valid", "empty", "tampered", "invalid", "cancel"] {
        let cwd = tempfile::tempdir().unwrap();
        let source = Solid::box_solid(40., 20., 10.).unwrap();
        let hash = stage(cwd.path(), &source);
        let original = std::fs::read(cwd.path().join("original.step")).unwrap();
        if scenario == "tampered" {
            std::fs::write(
                cwd.path()
                    .join(forma_core::native::assets::relative_path(&hash)),
                b"tampered",
            )
            .unwrap();
        }
        let normal = if scenario == "invalid" {
            [0.; 3]
        } else {
            [0., 0., 1.]
        };
        let origin = if scenario == "empty" {
            [0., 0., 50.]
        } else {
            [0., 0., 5.]
        };
        let result = section_with_executable(
            cwd.path(),
            &hash,
            &plane(origin, normal),
            Arc::new(AtomicBool::new(scenario == "cancel")),
            worker,
        )
        .await;
        match scenario {
            "valid" => {
                let result = result.unwrap();
                close(result.geometry.total_length_mm, 120.);
                assert_eq!(result.source_sha256, hash);
            }
            "empty" => assert!(result.unwrap().geometry.curves.is_empty()),
            _ => {
                assert!(result.is_err());
                assert!(!cwd.path().join("section.json").exists());
            }
        }
        assert_eq!(
            std::fs::read(cwd.path().join("original.step")).unwrap(),
            original
        );
        for path in ["model.step", "preview.glb"] {
            assert!(!cwd.path().join(path).exists());
        }
    }
}
