use forma_core::{
    cad_ir::{Document, Operation, Parameter},
    sketch_analysis::analyze_operation,
};
fn fixture() -> Document {
    serde_json::from_str(include_str!(
        "../../../../docs/fixtures/sketch-holes.cad.json"
    ))
    .unwrap()
}
#[test]
fn analysis_reports_hole_area_without_mutating_the_operation() {
    let document = fixture();
    let operation = document.features[0].operation.clone();
    let before = serde_json::to_value(&operation).unwrap();
    let report = analyze_operation(operation.clone(), vec![], "profile").unwrap();
    assert_eq!(report.status, "solved");
    assert_eq!(report.loop_count, 3);
    assert_eq!(report.hole_count, 2);
    assert_eq!(report.degrees_of_freedom, Some(24));
    assert_eq!(report.profile_area_mm2, Some(1104.0));
    assert_eq!(report.solved_points.len(), 12);
    assert_eq!(serde_json::to_value(operation).unwrap(), before);
}
#[test]
fn invalid_hole_layout_has_no_solved_preview_or_degree_claim() {
    let mut document = fixture();
    if let Operation::Sketch2d { points, .. } = &mut document.features[0].operation {
        points[4].x_mm = -2.0;
    }
    let report =
        analyze_operation(document.features[0].operation.clone(), vec![], "profile").unwrap();
    assert_eq!(report.status, "invalidProfile");
    assert!(report.solved_points.is_empty());
    assert!(report.degrees_of_freedom.is_none());
    assert!(report.profile_area_mm2.is_none());
}
#[test]
fn duplicate_and_nonfinite_parameter_inputs_are_rejected() {
    let operation = fixture().features[0].operation.clone();
    let parameter = Parameter {
        id: "p".into(),
        name: "Value".into(),
        value_mm: 2.0,
    };
    assert!(analyze_operation(
        operation.clone(),
        vec![parameter.clone(), parameter.clone()],
        "profile"
    )
    .is_err());
    assert!(analyze_operation(
        operation,
        vec![Parameter {
            value_mm: f64::INFINITY,
            ..parameter
        }],
        "profile"
    )
    .is_err());
}
