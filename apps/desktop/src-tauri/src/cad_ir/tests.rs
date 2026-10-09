use super::*;

#[test]
fn suppression_is_atomic_and_limited_to_single_input_modifiers() {
    let mut current = plate();
    current.features.push(Feature {
        id: "rounding".into(),
        name: "Rounding".into(),
        operation: Operation::Fillet {
            body_feature_id: "extrude".into(),
            radius: Dimension::Literal { mm: 1.0 },
        },
        suppressed: false,
    });
    current.bodies[0].source_feature_id = "rounding".into();
    let updated = apply_commands(
        &current,
        "revision_1",
        "revision_2",
        &[Command::SetFeatureSuppressed {
            feature_id: "rounding".into(),
            suppressed: true,
        }],
    )
    .unwrap();
    assert!(updated.features[2].suppressed);
    assert_eq!(updated.features[2].id, "rounding");
    assert!(!current.features[2].suppressed);
    let error = apply_commands(
        &current,
        "revision_1",
        "revision_3",
        &[Command::SetFeatureSuppressed {
            feature_id: "extrude".into(),
            suppressed: true,
        }],
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::UnsupportedSuppression);
}

#[test]
fn cone_tip_can_be_zero_but_equal_or_negative_radii_are_rejected() {
    let mut document = Document {
        schema_version: SCHEMA_VERSION,
        revision_id: "revision_cone".into(),
        parameters: vec![Parameter {
            id: "tip".into(),
            name: "Tip radius".into(),
            value_mm: 0.0,
        }],
        features: vec![Feature {
            id: "cone".into(),
            name: "Cone".into(),
            suppressed: false,
            operation: Operation::Cone {
                bottom_radius: Dimension::Literal { mm: 5.0 },
                top_radius: Dimension::Parameter {
                    parameter_id: "tip".into(),
                },
                height: Dimension::Literal { mm: 10.0 },
            },
        }],
        bodies: vec![Body {
            id: "body".into(),
            name: "Cone".into(),
            source_feature_id: "cone".into(),
        }],
    };
    assert!(document.validate().is_ok());
    document.parameters[0].value_mm = 5.0;
    assert_eq!(
        document.validate().unwrap_err().code,
        ErrorCode::InvalidValue
    );
    document.parameters[0].value_mm = -1.0;
    assert_eq!(
        document.validate().unwrap_err().code,
        ErrorCode::InvalidValue
    );
}

fn plate() -> Document {
    Document {
        schema_version: SCHEMA_VERSION,
        revision_id: "revision_1".into(),
        parameters: vec![Parameter {
            id: "width".into(),
            name: "Width".into(),
            value_mm: 80.0,
        }],
        features: vec![
            Feature {
                id: "sketch".into(),
                name: "Sketch".into(),
                suppressed: false,
                operation: Operation::Rectangle {
                    width: Dimension::Parameter {
                        parameter_id: "width".into(),
                    },
                    depth: Dimension::Literal { mm: 40.0 },
                },
            },
            Feature {
                id: "extrude".into(),
                name: "Extrude".into(),
                suppressed: false,
                operation: Operation::Extrude {
                    sketch_id: "sketch".into(),
                    distance: Dimension::Literal { mm: 10.0 },
                },
            },
        ],
        bodies: vec![Body {
            id: "plate".into(),
            name: "Plate".into(),
            source_feature_id: "extrude".into(),
        }],
    }
}

#[test]
fn named_hole_checks_its_source_and_dimensions() {
    let mut document = plate();
    document.features.push(Feature {
        id: "mounting_hole".into(),
        name: "Mounting hole".into(),
        suppressed: false,
        operation: Operation::Hole {
            body_feature_id: "extrude".into(),
            center_mm: [20.0, 20.0],
            start_z_mm: 0.0,
            radius: Dimension::Literal { mm: 3.0 },
            depth: Dimension::Literal { mm: 10.0 },
        },
    });
    document.bodies[0].source_feature_id = "mounting_hole".into();
    assert!(document.validate().is_ok());
    if let Operation::Hole { depth, .. } = &mut document.features[2].operation {
        *depth = Dimension::Literal { mm: 0.0 };
    }
    assert_eq!(
        document.validate().unwrap_err().code,
        ErrorCode::InvalidValue
    );
}

#[test]
fn fillet_has_a_stable_source_and_editable_radius() {
    let mut document = plate();
    document.features.push(Feature {
        id: "soft_edges".into(),
        name: "Soft edges".into(),
        suppressed: false,
        operation: Operation::Fillet {
            body_feature_id: "extrude".into(),
            radius: Dimension::Literal { mm: 2.0 },
        },
    });
    document.bodies[0].source_feature_id = "soft_edges".into();
    assert!(document.validate().is_ok());
    let edited = apply_commands(
        &document,
        "revision_1",
        "revision_2",
        &[Command::SetLiteral {
            feature_id: "soft_edges".into(),
            field: DimensionField::Radius,
            value_mm: 3.0,
        }],
    )
    .unwrap();
    assert!(matches!(
        &edited.features[2].operation,
        Operation::Fillet {
            radius: Dimension::Literal { mm: 3.0 },
            ..
        }
    ));
    assert_eq!(
        apply_commands(
            &document,
            "revision_1",
            "revision_2",
            &[Command::SetLiteral {
                feature_id: "soft_edges".into(),
                field: DimensionField::Radius,
                value_mm: 0.0,
            }],
        )
        .unwrap_err()
        .code,
        ErrorCode::InvalidValue
    );
    if let Operation::Fillet {
        body_feature_id, ..
    } = &mut document.features[2].operation
    {
        *body_feature_id = "missing".into();
    }
    assert_eq!(
        document.validate().unwrap_err().code,
        ErrorCode::BrokenReference
    );
}

#[test]
fn chamfer_command_extends_an_existing_body_atomically() {
    let current = plate();
    let chamfer = Feature {
        id: "bevel".into(),
        name: "Edge bevel".into(),
        suppressed: false,
        operation: Operation::Chamfer {
            body_feature_id: "extrude".into(),
            distance: Dimension::Literal { mm: 2.0 },
        },
    };
    let edited = apply_commands(
        &current,
        "revision_1",
        "revision_2",
        &[
            Command::AddFeature { feature: chamfer },
            Command::SetBodySource {
                body_id: "plate".into(),
                source_feature_id: "bevel".into(),
            },
        ],
    )
    .unwrap();
    assert_eq!(edited.bodies[0].source_feature_id, "bevel");
    assert_eq!(current.bodies[0].source_feature_id, "extrude");
    assert_eq!(
        apply_commands(
            &current,
            "revision_1",
            "revision_2",
            &[Command::SetBodySource {
                body_id: "plate".into(),
                source_feature_id: "missing".into(),
            }],
        )
        .unwrap_err()
        .code,
        ErrorCode::InvalidBody
    );
}

#[test]
fn command_batch_is_atomic_and_checks_revision() {
    let current = plate();
    let commands = [
        Command::SetParameter {
            parameter_id: "width".into(),
            value_mm: 120.0,
        },
        Command::SetLiteral {
            feature_id: "extrude".into(),
            field: DimensionField::Distance,
            value_mm: 0.0,
        },
    ];
    let error = apply_commands(&current, "revision_1", "revision_2", &commands).unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidValue);
    assert_eq!(current.parameters[0].value_mm, 80.0);
    assert_eq!(
        apply_commands(&current, "stale", "revision_2", &commands)
            .unwrap_err()
            .code,
        ErrorCode::RevisionConflict
    );
}

#[test]
fn valid_batch_preserves_ids_and_changes_revision() {
    let current = plate();
    let changed = apply_commands(
        &current,
        "revision_1",
        "revision_2",
        &[
            Command::SetParameter {
                parameter_id: "width".into(),
                value_mm: 120.0,
            },
            Command::RenameFeature {
                feature_id: "extrude".into(),
                name: "Base plate".into(),
            },
        ],
    )
    .unwrap();
    assert_eq!(changed.revision_id, "revision_2");
    assert_eq!(changed.features[1].id, "extrude");
    assert_eq!(changed.bodies[0].source_feature_id, "extrude");
    assert_eq!(changed.parameters[0].value_mm, 120.0);
    assert_eq!(current.revision_id, "revision_1");
}

#[test]
fn rejects_broken_dependencies_and_duplicate_semantic_ids() {
    let mut document = plate();
    document.features.swap(0, 1);
    assert_eq!(
        document.validate().unwrap_err().code,
        ErrorCode::BrokenReference
    );
    let mut document = plate();
    document.bodies[0].id = "width".into();
    assert_eq!(
        document.validate().unwrap_err().code,
        ErrorCode::DuplicateId
    );
}

#[test]
fn parameter_binding_cannot_be_silently_replaced_with_literal() {
    let current = plate();
    let error = apply_commands(
        &current,
        "revision_1",
        "revision_2",
        &[Command::SetLiteral {
            feature_id: "sketch".into(),
            field: DimensionField::Width,
            value_mm: 60.0,
        }],
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::ParameterBound);
}

#[test]
fn rejects_unknown_fields_and_nonfinite_values() {
    let raw = serde_json::to_string(&plate()).unwrap();
    let mut value: serde_json::Value = serde_json::from_str(&raw).unwrap();
    value["unexpected"] = serde_json::json!(true);
    assert!(serde_json::from_value::<Document>(value).is_err());
    let mut document = plate();
    document.parameters[0].value_mm = f64::NAN;
    assert_eq!(
        document.validate().unwrap_err().code,
        ErrorCode::InvalidValue
    );
}

#[test]
fn creates_parameter_feature_and_body_in_one_batch() {
    let current = Document {
        schema_version: SCHEMA_VERSION,
        revision_id: "revision_1".into(),
        parameters: vec![],
        features: vec![],
        bodies: vec![],
    };
    let changed = apply_commands(
        &current,
        "revision_1",
        "revision_2",
        &[
            Command::AddParameter {
                parameter: Parameter {
                    id: "radius".into(),
                    name: "Radius".into(),
                    value_mm: 12.0,
                },
            },
            Command::AddFeature {
                feature: Feature {
                    id: "profile".into(),
                    name: "Circle".into(),
                    suppressed: false,
                    operation: Operation::Circle {
                        radius: Dimension::Parameter {
                            parameter_id: "radius".into(),
                        },
                    },
                },
            },
            Command::AddFeature {
                feature: Feature {
                    id: "pad".into(),
                    name: "Extrude".into(),
                    suppressed: false,
                    operation: Operation::Extrude {
                        sketch_id: "profile".into(),
                        distance: Dimension::Literal { mm: 5.0 },
                    },
                },
            },
            Command::AddBody {
                body: Body {
                    id: "body".into(),
                    name: "Disk".into(),
                    source_feature_id: "pad".into(),
                },
            },
        ],
    )
    .unwrap();
    assert_eq!(changed.features.len(), 2);
    assert_eq!(changed.bodies.len(), 1);
    assert!(current.features.is_empty());
}

#[test]
fn command_batch_rebuilds_existing_geometry_from_parameter() {
    let current = plate();
    let before = current.compile_legacy_geometry().unwrap();
    let changed = apply_commands(
        &current,
        "revision_1",
        "revision_2",
        &[Command::SetParameter {
            parameter_id: "width".into(),
            value_mm: 120.0,
        }],
    )
    .unwrap();
    let after = compile_json(&serde_json::to_string(&changed).unwrap()).unwrap();
    assert!(before.source.contains("rect(80,40).extrude(10)"));
    assert!(after.source.contains("rect(120,40).extrude(10)"));
    assert_eq!(after.lines[0].feature_id, "sketch");
    assert_eq!(after.lines[1].feature_id, "extrude");
    assert_eq!(current.parameters[0].value_mm, 80.0);
}

#[test]
fn adapter_rejects_multiple_bodies_instead_of_dropping_geometry() {
    let mut document = plate();
    document.bodies.push(Body {
        id: "second_body".into(),
        name: "Second body".into(),
        source_feature_id: "extrude".into(),
    });
    assert!(document.validate().is_ok());
    assert!(document.compile_legacy_geometry().is_err());
}

#[test]
fn existing_typed_documents_remain_accepted() {
    let source = include_str!("../../../../../docs/fixtures/plate-hole.cad.json");
    assert_eq!(
        compile_json(source).unwrap().source,
        crate::cad_document::CadDocument::parse(source)
            .unwrap()
            .compile()
            .unwrap()
            .source
    );
}

#[test]
fn ir_document_builds_real_geometry_when_kernel_is_available() {
    let Ok(python) = std::env::var("FORMA_TEST_PYTHON") else {
        return;
    };
    use std::{
        io::Write,
        process::{Command as ProcessCommand, Stdio},
    };
    let compiled = plate().compile_legacy_geometry().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let mut child = ProcessCommand::new(python)
        .args(["-I", "-c", include_str!("../../scripts/model_program.py")])
        .current_dir(dir.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(
            serde_json::json!({"program":compiled.source,"features":compiled.lines})
                .to_string()
                .as_bytes(),
        )
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!((report["volume"].as_f64().unwrap() - 32000.0).abs() < 0.001);
    assert!(dir.path().join("model.step").is_file());
}
