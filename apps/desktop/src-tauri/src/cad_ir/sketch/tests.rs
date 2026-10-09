use super::*;
use crate::cad_ir::{
    apply_commands, Body, Command, Document, Feature, Operation, Parameter, SCHEMA_VERSION,
};
fn rectangle() -> (Vec<SketchPoint>, Vec<SketchLine>, Vec<SketchConstraint>) {
    let points = [
        ("a", 0.0, 0.0),
        ("b", 19.0, 0.2),
        ("c", 19.8, 9.5),
        ("d", -0.1, 9.8),
    ]
    .into_iter()
    .map(|(id, x_mm, y_mm)| SketchPoint {
        id: id.into(),
        x_mm,
        y_mm,
    })
    .collect();
    let lines = [
        ("ab", "a", "b"),
        ("bc", "b", "c"),
        ("cd", "c", "d"),
        ("da", "d", "a"),
    ]
    .into_iter()
    .map(|(id, start_point_id, end_point_id)| SketchLine {
        id: id.into(),
        start_point_id: start_point_id.into(),
        end_point_id: end_point_id.into(),
        construction: false,
    })
    .collect();
    let mut constraints = vec![SketchConstraint::Fixed {
        id: "anchor".into(),
        point_id: "a".into(),
        x_mm: 0.0,
        y_mm: 0.0,
    }];
    for (id, line_id, kind) in [
        ("h1", "ab", true),
        ("v1", "bc", false),
        ("h2", "cd", true),
        ("v2", "da", false),
    ] {
        constraints.push(if kind {
            SketchConstraint::Horizontal {
                id: id.into(),
                line_id: line_id.into(),
            }
        } else {
            SketchConstraint::Vertical {
                id: id.into(),
                line_id: line_id.into(),
            }
        });
    }
    constraints.extend([
        SketchConstraint::Length {
            id: "width".into(),
            line_id: "ab".into(),
            distance: Dimension::Parameter {
                parameter_id: "w".into(),
            },
        },
        SketchConstraint::Length {
            id: "height".into(),
            line_id: "bc".into(),
            distance: Dimension::Literal { mm: 10.0 },
        },
    ]);
    (points, lines, constraints)
}
#[test]
fn solves_parametric_rectangle_from_imperfect_draft() {
    let (points, lines, constraints) = rectangle();
    let solved = solve(
        SketchPlane::Xy,
        [0.0; 3],
        &points,
        &lines,
        &constraints,
        &HashMap::from([("w", 20.0)]),
        "sketch",
    )
    .unwrap();
    assert!((solved.outline_xy[2] - 20.0).abs() < 1e-5);
    assert!((solved.outline_xy[5] - 10.0).abs() < 1e-5);
}
#[test]
fn rejects_conflicting_dimensions() {
    let (points, lines, mut constraints) = rectangle();
    constraints.push(SketchConstraint::Length {
        id: "conflict".into(),
        line_id: "ab".into(),
        distance: Dimension::Literal { mm: 30.0 },
    });
    assert_eq!(
        solve(
            SketchPlane::Xy,
            [0.0; 3],
            &points,
            &lines,
            &constraints,
            &HashMap::from([("w", 20.0)]),
            "sketch"
        )
        .unwrap_err()
        .code,
        ErrorCode::SketchConstraintConflict
    );
}
#[test]
fn rejects_outline_touching_a_nonadjacent_edge() {
    let points = [
        ("a", 0.0, 0.0),
        ("b", 10.0, 0.0),
        ("c", 10.0, 10.0),
        ("d", 5.0, 0.0),
        ("e", 0.0, 10.0),
    ]
    .into_iter()
    .map(|(id, x_mm, y_mm)| SketchPoint {
        id: id.into(),
        x_mm,
        y_mm,
    })
    .collect::<Vec<_>>();
    let lines = [
        ("ab", "a", "b"),
        ("bc", "b", "c"),
        ("cd", "c", "d"),
        ("de", "d", "e"),
        ("ea", "e", "a"),
    ]
    .into_iter()
    .map(|(id, start_point_id, end_point_id)| SketchLine {
        id: id.into(),
        start_point_id: start_point_id.into(),
        end_point_id: end_point_id.into(),
        construction: false,
    })
    .collect::<Vec<_>>();
    assert_eq!(
        solve(
            SketchPlane::Xy,
            [0.0; 3],
            &points,
            &lines,
            &[],
            &HashMap::new(),
            "profile"
        )
        .unwrap_err()
        .code,
        ErrorCode::InvalidSketch
    );
}

#[test]
fn sketch_commands_are_atomic_and_keep_parameter_bindings() {
    let (points, lines, constraints) = rectangle();
    let current = Document {
        schema_version: SCHEMA_VERSION,
        revision_id: "current".into(),
        parameters: vec![Parameter {
            id: "w".into(),
            name: "Width".into(),
            value_mm: 20.0,
        }],
        features: vec![
            Feature {
                id: "profile".into(),
                name: "Profile".into(),
                suppressed: false,
                operation: Operation::Sketch2d {
                    plane: SketchPlane::Xy,
                    origin_mm: [0.0; 3],
                    points,
                    lines,
                    constraints,
                    bindings: vec![],
                },
            },
            Feature {
                id: "pad".into(),
                name: "Pad".into(),
                suppressed: false,
                operation: Operation::Extrude {
                    sketch_id: "profile".into(),
                    distance: Dimension::Literal { mm: 5.0 },
                },
            },
        ],
        bodies: vec![Body {
            id: "body".into(),
            name: "Body".into(),
            source_feature_id: "pad".into(),
        }],
    };
    let changed = apply_commands(
        &current,
        "current",
        "changed",
        &[Command::SetSketchLength {
            feature_id: "profile".into(),
            constraint_id: "height".into(),
            distance_mm: 12.0,
        }],
    )
    .unwrap();
    assert_eq!(changed.revision_id, "changed");
    assert_eq!(current.revision_id, "current");
    let bound = apply_commands(
        &current,
        "current",
        "bad",
        &[Command::SetSketchLength {
            feature_id: "profile".into(),
            constraint_id: "width".into(),
            distance_mm: 30.0,
        }],
    )
    .unwrap_err();
    assert_eq!(bound.code, ErrorCode::ParameterBound);
    let conflict = apply_commands(
        &current,
        "current",
        "bad",
        &[Command::AddSketchConstraint {
            feature_id: "profile".into(),
            constraint: SketchConstraint::Length {
                id: "impossible".into(),
                line_id: "ab".into(),
                distance: Dimension::Literal { mm: 30.0 },
            },
        }],
    )
    .unwrap();
    let Operation::Sketch2d {
        plane,
        origin_mm,
        points,
        lines,
        constraints,
        ..
    } = &conflict.features[0].operation
    else {
        unreachable!()
    };
    assert_eq!(
        solve(
            *plane,
            *origin_mm,
            points,
            lines,
            constraints,
            &HashMap::from([("w", 20.0)]),
            "profile"
        )
        .unwrap_err()
        .code,
        ErrorCode::SketchConstraintConflict
    );
}

#[test]
fn construction_diagonal_drives_profile_without_entering_outline() {
    let (points, mut lines, mut constraints) = rectangle();
    constraints.retain(|constraint| constraint.id() != "height");
    lines.insert(
        0,
        SketchLine {
            id: "diagonal".into(),
            start_point_id: "a".into(),
            end_point_id: "c".into(),
            construction: true,
        },
    );
    constraints.push(SketchConstraint::Length {
        id: "diagonal_length".into(),
        line_id: "diagonal".into(),
        distance: Dimension::Literal {
            mm: 1300.0_f64.sqrt(),
        },
    });
    let solved = solve(
        SketchPlane::Xy,
        [0.0; 3],
        &points,
        &lines,
        &constraints,
        &HashMap::from([("w", 20.0)]),
        "profile",
    )
    .unwrap();
    assert_eq!(solved.outline_xy.len(), 8);
    assert!((solved.outline_xy[2] - 20.0).abs() < 1e-5);
    assert!((solved.outline_xy[5] - 30.0).abs() < 1e-5);
}

#[test]
fn independent_construction_endpoints_and_crossing_guides_are_not_profile_vertices() {
    let (mut points, mut lines, constraints) = rectangle();
    points.insert(
        0,
        SketchPoint {
            id: "guide_start".into(),
            x_mm: -100.0,
            y_mm: 5.0,
        },
    );
    points.push(SketchPoint {
        id: "guide_end".into(),
        x_mm: 100.0,
        y_mm: 5.0,
    });
    lines.insert(
        0,
        SketchLine {
            id: "guide".into(),
            start_point_id: "guide_start".into(),
            end_point_id: "guide_end".into(),
            construction: true,
        },
    );
    let solved = solve(
        SketchPlane::Xy,
        [0.0; 3],
        &points,
        &lines,
        &constraints,
        &HashMap::from([("w", 20.0)]),
        "profile",
    )
    .unwrap();
    assert_eq!(solved.outline_xy.len(), 8);
    assert!((solved.outline_xy[0]).abs() < 1e-5);
    assert!((solved.outline_xy[5] - 10.0).abs() < 1e-5);
}

#[test]
fn invalid_construction_and_open_or_multiple_profiles_are_rejected() {
    let (points, mut lines, constraints) = rectangle();
    let parameters = HashMap::from([("w", 20.0)]);
    lines[0].construction = true;
    assert!(validate_structure(
        [0.0; 3],
        &points,
        &lines,
        &constraints,
        &parameters,
        "profile"
    )
    .is_err());
    lines[0].construction = false;
    let mut orphan = points.clone();
    orphan.push(SketchPoint {
        id: "orphan".into(),
        x_mm: 1.0,
        y_mm: 1.0,
    });
    assert!(validate_structure(
        [0.0; 3],
        &orphan,
        &lines,
        &constraints,
        &parameters,
        "profile"
    )
    .is_err());
    lines.push(SketchLine {
        id: "bad".into(),
        start_point_id: "a".into(),
        end_point_id: "missing".into(),
        construction: true,
    });
    assert!(validate_structure(
        [0.0; 3],
        &points,
        &lines,
        &constraints,
        &parameters,
        "profile"
    )
    .is_err());
    lines.last_mut().unwrap().end_point_id = "a".into();
    assert!(validate_structure(
        [0.0; 3],
        &points,
        &lines,
        &constraints,
        &parameters,
        "profile"
    )
    .is_err());
    lines.last_mut().unwrap().end_point_id = "b".into();
    while lines.len() < 65 {
        let mut line = lines.last().unwrap().clone();
        line.id = format!("guide_{}", lines.len());
        lines.push(line);
    }
    assert!(validate_structure(
        [0.0; 3],
        &points,
        &lines,
        &constraints,
        &parameters,
        "profile"
    )
    .is_err());
    let points: Vec<_> = (0..6)
        .map(|i| SketchPoint {
            id: format!("p{i}"),
            x_mm: i as f64,
            y_mm: 0.0,
        })
        .collect();
    let lines: Vec<_> = (0..6)
        .map(|i| SketchLine {
            id: format!("l{i}"),
            start_point_id: format!("p{i}"),
            end_point_id: format!(
                "p{}",
                if i == 2 {
                    0
                } else if i == 5 {
                    3
                } else {
                    i + 1
                }
            ),
            construction: false,
        })
        .collect();
    assert!(solve(
        SketchPlane::Xy,
        [0.0; 3],
        &points,
        &lines,
        &[],
        &HashMap::new(),
        "profile"
    )
    .is_err());
}

#[test]
fn legacy_lines_default_to_profile_and_construction_conflicts_are_rejected() {
    let line: SketchLine =
        serde_json::from_str(r#"{"id":"ab","startPointId":"a","endPointId":"b"}"#).unwrap();
    assert!(!line.construction);
    let (points, mut lines, mut constraints) = rectangle();
    lines.push(SketchLine {
        id: "diagonal".into(),
        start_point_id: "a".into(),
        end_point_id: "c".into(),
        construction: true,
    });
    constraints.push(SketchConstraint::Length {
        id: "diagonal_length".into(),
        line_id: "diagonal".into(),
        distance: Dimension::Literal { mm: 100.0 },
    });
    assert_eq!(
        solve(
            SketchPlane::Xy,
            [0.0; 3],
            &points,
            &lines,
            &constraints,
            &HashMap::from([("w", 20.0)]),
            "profile"
        )
        .unwrap_err()
        .code,
        ErrorCode::SketchConstraintConflict
    );
}
