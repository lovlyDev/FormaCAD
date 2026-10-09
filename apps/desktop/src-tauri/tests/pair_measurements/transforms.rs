use super::fixture::*;

#[tokio::test]
async fn resized_rotated_translated_mirrored_box_pair_values_and_occurrence_echo_remain_exact() {
    for width in [40., 64.] {
        for (kind, a, b, expected) in [
            ("minimumDistance", "box-face:xmin", "box-face:xmax", width),
            ("faceNormalAngle", "box-face:zmin", "box-face:zmax", 180.),
            (
                "edgeAcuteAngle",
                "box-edge:x:ymin:zmin",
                "box-edge:y:xmin:zmin",
                90.,
            ),
        ] {
            let (dir, binding) = stage(&document(false, width, 10., true), &[]);
            let q = pair(kind, a, b, true);
            let report = measure(dir.path(), &binding, &q).await.unwrap();
            assert_eq!(report.query, q);
            close(scalar(&report.result), expected);
            no_geometry(dir.path());
        }
    }
}

#[tokio::test]
async fn resized_transformed_cylinder_caps_curved_side_and_mixed_entities_use_exact_distances() {
    for (radius, height) in [(8., 15.), (12., 21.)] {
        for transformed in [false, true] {
            for (kind, a, b, expected) in [
                (
                    "minimumDistance",
                    "cylinder-face:bottom",
                    "cylinder-face:top",
                    height,
                ),
                (
                    "minimumDistance",
                    "cylinder-edge:bottom",
                    "cylinder-edge:top",
                    height,
                ),
                (
                    "minimumDistance",
                    "cylinder-face:side",
                    "cylinder-edge:top",
                    0.,
                ),
                (
                    "minimumDistance",
                    "cylinder-face:bottom",
                    "cylinder-edge:top",
                    height,
                ),
                (
                    "faceNormalAngle",
                    "cylinder-face:bottom",
                    "cylinder-face:top",
                    180.,
                ),
            ] {
                let (dir, binding) = stage(&document(true, radius, height, transformed), &[]);
                let q = pair(kind, a, b, transformed);
                let report = measure(dir.path(), &binding, &q).await.unwrap();
                close(scalar(&report.result), expected);
                assert_eq!(report.query, q);
                no_geometry(dir.path());
            }
        }
    }
}
