use super::*;
use serde_json::json;

#[test]
fn circular_diameter_is_an_edge_query_with_a_strict_positive_matching_result() {
    let value = json!({"kind":"edgeDiameter","reference":{"schemaVersion":1,"kind":"edge",
        "ownerFeatureId":"pad","role":"cylinder-edge:top","occurrencePath":["turn"]}});
    let query: MeasurementQuery = serde_json::from_value(value.clone()).unwrap();
    assert!(query.valid());
    assert_eq!(query.tag(), 5);
    assert!(MeasurementValue::EdgeDiameter { diameter_mm: 20. }.valid_for(&query));
    assert!(!MeasurementValue::EdgeRadius { radius_mm: 10. }.valid_for(&query));
    for diameter_mm in [0., -1., f64::NAN, f64::INFINITY] {
        assert!(!MeasurementValue::EdgeDiameter { diameter_mm }.valid_for(&query));
    }
    let mut wrong = value.clone();
    wrong["reference"]["kind"] = "face".into();
    assert!(!serde_json::from_value::<MeasurementQuery>(wrong)
        .unwrap()
        .valid());
    for extra in ["ordinal", "diameterMm", "sourcePath"] {
        let mut wrong = value.clone();
        wrong[extra] = 1.into();
        assert!(serde_json::from_value::<MeasurementQuery>(wrong).is_err());
    }
    let mut seam = value;
    seam["reference"]["role"] = "cylinder-edge:seam".into();
    assert!(!serde_json::from_value::<MeasurementQuery>(seam)
        .unwrap()
        .valid());
}

fn face_query() -> MeasurementQuery {
    serde_json::from_value(json!({"kind":"planarFace","reference":{"schemaVersion":1,
        "kind":"face","ownerFeatureId":"pad","role":"box-face:zmax","occurrencePath":["turn"]}}))
    .unwrap()
}

#[test]
fn reference_queries_reject_wrong_kind_and_ordinal_payloads() {
    let query = face_query();
    assert!(query.valid());
    let mut value = serde_json::to_value(&query).unwrap();
    value["reference"]["kind"] = "edge".into();
    let mismatch: MeasurementQuery = serde_json::from_value(value.clone()).unwrap();
    assert!(!mismatch.valid());
    value["ordinal"] = 1.into();
    assert!(serde_json::from_value::<MeasurementQuery>(value).is_err());
    assert!(serde_json::from_value::<MeasurementQuery>(
        json!({"kind":"bodyMetrics","reference":{}})
    )
    .is_err());
}

#[test]
fn planar_result_requires_matching_kind_finite_origin_and_oriented_unit_normal() {
    let query = face_query();
    let valid = MeasurementValue::PlanarFace {
        area_mm2: 800.,
        origin_mm: [0., 0., 10.],
        normal: [0., 0., 1.],
    };
    assert!(valid.valid_for(&query));
    assert!(!MeasurementValue::FaceArea { area_mm2: 800. }.valid_for(&query));
    for normal in [[0., 0., 0.], [0., 0., 2.], [f64::NAN, 0., 1.]] {
        assert!(!MeasurementValue::PlanarFace {
            area_mm2: 800.,
            origin_mm: [0., 0., 10.],
            normal
        }
        .valid_for(&query));
    }
    assert!(!MeasurementValue::PlanarFace {
        area_mm2: 800.,
        origin_mm: [f64::INFINITY, 0., 10.],
        normal: [0., 0., 1.]
    }
    .valid_for(&query));
    // Validation preserves the supplied direction. Independent geometry tests must
    // establish which of the two unit signs is the actual outward BREP normal.
    assert_eq!(
        serde_json::to_value(valid).unwrap()["normal"],
        json!([0., 0., 1.])
    );
}

#[test]
fn null_or_zero_radius_is_not_a_successful_exact_circular_measurement() {
    let mut query = serde_json::to_value(face_query()).unwrap();
    query["kind"] = "edgeRadius".into();
    query["reference"]["kind"] = "edge".into();
    query["reference"]["role"] = "box-edge:x:ymin:zmax".into();
    let query: MeasurementQuery = serde_json::from_value(query).unwrap();
    assert!(!MeasurementValue::EdgeRadius { radius_mm: 0. }.valid_for(&query));
    assert!(serde_json::from_value::<MeasurementValue>(
        json!({"kind":"edgeRadius","radiusMm":null})
    )
    .is_err());
}
