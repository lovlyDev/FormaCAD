use super::*;
use serde_json::json;
fn query() -> FaceSectionQuery {
    serde_json::from_value(json!({"reference":{"schemaVersion":1,"kind":"face","ownerFeatureId":"pad","role":"box-face:zmax","occurrencePath":[]},"offsetMm":-5,"deflectionMm":0.01})).unwrap()
}
#[test]
fn query_refuses_renderer_plane_and_invalid_face_or_numeric_input() {
    let valid = query();
    assert!(valid.valid());
    for field in ["originMm", "normal", "sourcePath", "ordinal"] {
        let mut value = serde_json::to_value(&valid).unwrap();
        value[field] = json!([0, 0, 1]);
        assert!(serde_json::from_value::<FaceSectionQuery>(value).is_err());
    }
    for offset_mm in [f64::NAN, f64::INFINITY, 10001.] {
        assert!(!FaceSectionQuery {
            offset_mm,
            ..valid.clone()
        }
        .valid());
    }
    let mut wrong = valid.clone();
    wrong.reference.kind = TopologyKind::Edge;
    assert!(!wrong.valid());
}
#[test]
fn signed_plane_offset_and_empty_section_are_valid_but_forged_planes_are_not() {
    let query = query();
    let face = ResolvedFace {
        area_mm2: 800.,
        origin_mm: [0., 0., 10.],
        normal: [0., 0., 1.],
    };
    let geometry = FaceSectionGeometry {
        plane: face.plane(&query),
        curves: vec![],
        total_length_mm: 0.,
    };
    assert_eq!(geometry.plane.origin_mm, [0., 0., 5.]);
    assert!(geometry.valid_for(&face, &query));
    let bottom = ResolvedFace {
        area_mm2: 800.,
        origin_mm: [0., 0., 0.],
        normal: [0., 0., -1.],
    };
    assert_eq!(bottom.plane(&query).origin_mm, [0., 0., 5.]);
    let mut shifted = geometry.clone();
    shifted.plane.origin_mm[2] += 1.;
    assert!(!shifted.valid_for(&face, &query));
    let mut flipped = geometry.clone();
    flipped.plane.normal = [0., 0., -1.];
    assert!(!flipped.valid_for(&face, &query));
    let mut forged_total = geometry;
    forged_total.total_length_mm = 1.;
    assert!(!forged_total.valid_for(&face, &query));
}
#[test]
fn final_world_origin_is_bounded_without_clamping() {
    let face = ResolvedFace {
        area_mm2: 1.,
        origin_mm: [0., 0., 9999.],
        normal: [0., 0., 1.],
    };
    let query = FaceSectionQuery {
        offset_mm: 2.,
        ..query()
    };
    assert!(query.valid());
    assert!(face.valid_for(&query));
    let plane = face.plane(&query);
    assert_eq!(plane.origin_mm[2], 10001.);
    assert!(!plane.valid());
}
