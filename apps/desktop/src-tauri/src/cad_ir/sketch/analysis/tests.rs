use super::*;
use crate::{
    cad_ir::{Document, Operation},
    sketch_analysis::analyze_operation,
};
fn example() -> Document {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../docs/fixtures/sketch-construction.cad.json"
    )))
    .unwrap()
}
#[test]
fn fully_constrained_and_free_rectangles_report_local_rank_and_area() {
    let doc = example();
    let result = analyze_operation(
        doc.features[0].operation.clone(),
        doc.parameters.clone(),
        "profile",
    )
    .unwrap();
    assert_eq!(result.status, "solved");
    assert_eq!(result.degrees_of_freedom, Some(0));
    assert!((result.profile_area_mm2.unwrap() - 600.0).abs() < 1e-5);
    assert!(result.constraints.iter().all(|c| c.satisfied));
    let mut free = doc.features[0].operation.clone();
    if let Operation::Sketch2d { constraints, .. } = &mut free {
        constraints.clear();
    }
    let result = analyze_operation(free, doc.parameters, "profile").unwrap();
    assert_eq!(result.degrees_of_freedom, Some(8));
}
#[test]
fn conflicts_report_residuals_without_solved_preview() {
    let mut doc = example();
    doc.parameters[1].value_mm = 10.0;
    let result =
        analyze_operation(doc.features[0].operation.clone(), doc.parameters, "profile").unwrap();
    assert_eq!(result.status, "conflict");
    assert_eq!(result.error_code, Some(ErrorCode::SketchConstraintConflict));
    assert!(result.constraints.iter().any(|c| !c.satisfied));
    assert!(result.solved_points.is_empty());
    assert_eq!(result.degrees_of_freedom, None);
}
#[test]
fn redundant_equations_are_reported_without_rejecting_consistent_geometry() {
    let mut doc = example();
    if let Operation::Sketch2d { constraints, .. } = &mut doc.features[0].operation {
        constraints.push(SketchConstraint::Horizontal {
            id: "extra_horizontal".into(),
            line_id: "ab".into(),
        });
    }
    let result =
        analyze_operation(doc.features[0].operation.clone(), doc.parameters, "profile").unwrap();
    assert_eq!(result.status, "solved");
    assert!(result.redundant_equations.unwrap() >= 1);
    assert_eq!(result.degrees_of_freedom, Some(0));
}
#[test]
fn invalid_references_and_non_sketch_inputs_do_not_execute_or_panic() {
    let mut doc = example();
    if let Operation::Sketch2d { lines, .. } = &mut doc.features[0].operation {
        lines[0].end_point_id = "missing".into();
    }
    let result =
        analyze_operation(doc.features[0].operation.clone(), doc.parameters, "profile").unwrap();
    assert_eq!(result.status, "invalidStructure");
    assert!(result.constraints.is_empty());
    assert!(analyze_operation(
        Operation::Circle {
            radius: Dimension::Literal { mm: 5.0 }
        },
        vec![],
        "circle"
    )
    .is_err());
}
