use super::*;
use serde_json::json;
pub(super) fn query(kind: &str, first: &str, second: &str) -> PairMeasurementQuery {
    let reference = |role: &str| json!({"schemaVersion":1,"kind":if role.contains("-face:"){"face"}else{"edge"},"ownerFeatureId":"pad","role":role,"occurrencePath":[]});
    serde_json::from_value(json!({"kind":kind,"first":reference(first),"second":reference(second)}))
        .unwrap()
}
#[test]
fn pairs_reject_renderer_scalar_points_vectors_and_unknown_fields() {
    let q = query("minimumDistance", "box-face:xmin", "box-face:xmax");
    assert!(q.valid());
    for field in [
        "ordinal",
        "distanceMm",
        "normal",
        "pointAMm",
        "secondBodyId",
    ] {
        let mut value = serde_json::to_value(&q).unwrap();
        value[field] = 1.into();
        assert!(serde_json::from_value::<PairMeasurementQuery>(value).is_err());
    }
    assert!(!query("faceNormalAngle", "box-edge:x:ymin:zmin", "box-face:xmax").valid());
    assert!(!query("edgeAcuteAngle", "box-face:xmin", "box-edge:x:ymin:zmin").valid());
}
#[test]
fn zero_distance_and_representative_witnesses_are_valid_but_inconsistent_distance_is_not() {
    let q = query("minimumDistance", "box-face:xmin", "box-face:xmin");
    assert!(PairMeasurementValue::MinimumDistance {
        distance_mm: 0.,
        point_a_mm: [-20., 0., 0.],
        point_b_mm: [-20., 0., 0.]
    }
    .valid_for(&q));
    assert!(PairMeasurementValue::MinimumDistance {
        distance_mm: 40.,
        point_a_mm: [-20., 0., 0.],
        point_b_mm: [20., 0., 0.]
    }
    .valid_for(&q));
    for distance_mm in [-1., f64::NAN, f64::INFINITY, 41.] {
        assert!(!PairMeasurementValue::MinimumDistance {
            distance_mm,
            point_a_mm: [-20., 0., 0.],
            point_b_mm: [20., 0., 0.]
        }
        .valid_for(&q));
    }
    assert!(!PairMeasurementValue::MinimumDistance {
        distance_mm: 0.,
        point_a_mm: [1e10, 0., 0.],
        point_b_mm: [1e10, 0., 0.]
    }
    .valid_for(&q));
}
#[test]
fn angle_conventions_preserve_zero_opposed_normals_and_distinct_ranges() {
    let faces = query("faceNormalAngle", "box-face:xmin", "box-face:xmax");
    let edges = query(
        "edgeAcuteAngle",
        "box-edge:x:ymin:zmin",
        "box-edge:x:ymax:zmax",
    );
    for angle_deg in [0., 90., 180.] {
        assert!(PairMeasurementValue::FaceNormalAngle { angle_deg }.valid_for(&faces));
    }
    for angle_deg in [0., 90.] {
        assert!(PairMeasurementValue::EdgeAcuteAngle { angle_deg }.valid_for(&edges));
    }
    assert!(!PairMeasurementValue::EdgeAcuteAngle { angle_deg: 180. }.valid_for(&edges));
    assert!(!PairMeasurementValue::FaceNormalAngle { angle_deg: 0. }.valid_for(&edges));
    for angle_deg in [-1., f64::NAN, f64::INFINITY, 181.] {
        assert!(!PairMeasurementValue::FaceNormalAngle { angle_deg }.valid_for(&faces));
    }
}
