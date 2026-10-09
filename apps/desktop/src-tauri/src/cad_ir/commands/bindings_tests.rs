use crate::cad_ir::sketch::bindings::{
    Axis, CoordinateBinding, ParameterCoordinate, ParameterKind,
};
use crate::cad_ir::{apply_commands, Command, Document, ErrorCode, Operation};
fn fixture() -> Document {
    serde_json::from_str(include_str!(
        "../../../../../../docs/fixtures/linked-profiles.cad.json"
    ))
    .unwrap()
}
fn command(value: serde_json::Value) -> Command {
    serde_json::from_value(value).unwrap()
}
#[test]
fn unlink_materializes_only_selected_axis_at_its_current_batch_value() {
    let current = fixture();
    let next = apply_commands(&current, "draft", "next", &[
        command(serde_json::json!({"command":"set_parameter","parameterId":"width","valueMm":80})),
        command(serde_json::json!({"command":"remove_sketch_binding","featureId":"base_sketch","bindingId":"a_x"})),
    ]).unwrap();
    let Operation::Sketch2d {
        points, bindings, ..
    } = &next.features[0].operation
    else {
        panic!()
    };
    assert_eq!(points[0].x_mm, -40.0);
    assert!(!bindings.iter().any(|binding| binding.id() == "a_x"));
    assert!(bindings.iter().any(|binding| binding.id() == "a_y"));
    let mut later = next.clone();
    later.parameters[0].value_mm = 100.0;
    let Operation::Sketch2d {
        origin_mm,
        points,
        constraints,
        bindings,
        ..
    } = &later.features[0].operation
    else {
        panic!()
    };
    let parameters = later
        .parameters
        .iter()
        .map(|p| (p.id.as_str(), p.value_mm))
        .collect();
    let (_, effective) = crate::cad_ir::sketch::bindings::resolve(
        *origin_mm,
        points,
        constraints,
        bindings,
        &parameters,
        "base_sketch",
    )
    .unwrap();
    assert_eq!(effective[0].x_mm, -40.0);
    assert_eq!(current.parameters[0].value_mm, 40.0);
    assert_eq!(next.revision_id, "next");
}
#[test]
fn origin_unlink_preserves_effective_world_placement_and_other_axes() {
    let current = fixture();
    let binding = CoordinateBinding::Origin {
        id: "origin_z".into(),
        axis: Axis::Z,
        value: ParameterCoordinate {
            kind: ParameterKind::Parameter,
            parameter_id: "base_height".into(),
            scale: 2.0,
            offset_mm: 1.0,
        },
    };
    let next = apply_commands(
        &current,
        "draft",
        "next",
        &[
            Command::AddSketchBinding {
                feature_id: "base_sketch".into(),
                binding,
            },
            Command::RemoveSketchBinding {
                feature_id: "base_sketch".into(),
                binding_id: "origin_z".into(),
            },
        ],
    )
    .unwrap();
    let Operation::Sketch2d {
        origin_mm,
        bindings,
        ..
    } = &next.features[0].operation
    else {
        panic!()
    };
    assert_eq!(*origin_mm, [0.0, 0.0, 21.0]);
    assert_eq!(bindings.len(), 8);
}
#[test]
fn binding_replacement_requires_existing_id_and_keeps_identity() {
    let current = fixture();
    let Operation::Sketch2d { bindings, .. } = &current.features[0].operation else {
        panic!()
    };
    let mut replacement = bindings[0].clone();
    if let CoordinateBinding::Point { value, .. } = &mut replacement {
        value.offset_mm = 2.0;
    }
    let next = apply_commands(
        &current,
        "draft",
        "next",
        &[Command::SetSketchBinding {
            feature_id: "base_sketch".into(),
            binding: replacement.clone(),
        }],
    )
    .unwrap();
    let Operation::Sketch2d { bindings, .. } = &next.features[0].operation else {
        panic!()
    };
    assert_eq!(bindings[0], replacement);
    if let CoordinateBinding::Point { id, .. } = &mut replacement {
        *id = "missing".into();
    }
    assert_eq!(
        apply_commands(
            &current,
            "draft",
            "next",
            &[Command::SetSketchBinding {
                feature_id: "base_sketch".into(),
                binding: replacement
            }]
        )
        .unwrap_err()
        .code,
        ErrorCode::MissingTarget
    );
}
#[test]
fn invalid_late_command_and_duplicate_targets_leave_original_untouched() {
    let current = fixture();
    let original = current.clone();
    let Operation::Sketch2d { bindings, .. } = &current.features[0].operation else {
        panic!()
    };
    let mut binding = bindings[0].clone();
    if let CoordinateBinding::Point { id, .. } = &mut binding {
        *id = "different_id".into();
    }
    for (last, code) in [
        (
            Command::AddSketchBinding {
                feature_id: "base_sketch".into(),
                binding,
            },
            ErrorCode::InvalidSketch,
        ),
        (
            Command::RemoveSketchBinding {
                feature_id: "base_sketch".into(),
                binding_id: "missing".into(),
            },
            ErrorCode::MissingTarget,
        ),
    ] {
        assert_eq!(
            apply_commands(
                &current,
                "draft",
                "next",
                &[
                    Command::SetParameter {
                        parameter_id: "width".into(),
                        value_mm: 80.0
                    },
                    last
                ]
            )
            .unwrap_err()
            .code,
            code
        );
        assert_eq!(current, original);
    }
}
#[test]
fn commands_obey_revision_limits_and_strict_json_shape() {
    let current = fixture();
    let edit = Command::SetParameter {
        parameter_id: "width".into(),
        value_mm: 80.0,
    };
    assert_eq!(
        apply_commands(&current, "stale", "next", std::slice::from_ref(&edit))
            .unwrap_err()
            .code,
        ErrorCode::RevisionConflict
    );
    assert_eq!(
        apply_commands(&current, "draft", "next", &vec![edit; 257])
            .unwrap_err()
            .code,
        ErrorCode::InvalidDocument
    );
    assert!(serde_json::from_value::<Command>(serde_json::json!({"command":"remove_sketch_binding","featureId":"base_sketch","bindingId":"a_x","deletePoint":true})).is_err());
}
