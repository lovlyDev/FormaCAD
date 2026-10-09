#![cfg(feature = "native-occt")]
use std::{
    io::Write,
    process::{Command, Stdio},
};
fn fixture() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../../docs/fixtures/linked-profiles.cad.json"
    ))
    .unwrap()
}
fn build(document: serde_json::Value) -> (serde_json::Value, tempfile::TempDir) {
    let cwd = tempfile::tempdir().unwrap();
    let request = serde_json::json!({"protocolVersion":1,"requestId":"links_test","bodyId":"body","document":document});
    let mut child = Command::new(env!("CARGO_BIN_EXE_forma-cad-worker"))
        .current_dir(cwd.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(request.to_string().as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    (
        serde_json::from_slice(&std::fs::read(cwd.path().join("result.json")).unwrap()).unwrap(),
        cwd,
    )
}
#[test]
fn shared_dimensions_rebuild_the_actual_joined_brep() {
    for (values, volume) in [
        ([40.0, 20.0, 10.0, 10.0], 12000.0),
        ([80.0, 30.0, 15.0, 12.0], 50400.0),
    ] {
        let mut doc = fixture();
        for (i, value) in values.iter().enumerate() {
            doc["parameters"][i]["valueMm"] = (*value).into();
        }
        let (result, cwd) = build(doc);
        assert_eq!(result["status"], "completed", "{result}");
        assert!(
            (result["volumeMm3"].as_f64().unwrap() - volume).abs() < 0.001,
            "{result}"
        );
        assert!(cwd.path().join("model.step").metadata().unwrap().len() > 100);
        assert!(cwd.path().join("preview.glb").metadata().unwrap().len() > 100);
    }
}
#[test]
fn invalid_or_conflicting_links_never_export_geometry() {
    for variant in 0..4 {
        let mut doc = fixture();
        let op = &mut doc["features"][0]["operation"];
        match variant {
            0 => op["bindings"][0]["value"]["parameterId"] = "missing".into(),
            1 => {
                let duplicate = op["bindings"][0].clone();
                op["bindings"].as_array_mut().unwrap().push(duplicate);
            }
            2 => op["bindings"][0]["value"]["scale"] = 9999.into(),
            _ => {
                op["constraints"] = serde_json::json!([{"id":"conflict","kind":"fixed","pointId":"a","xMm":0,"yMm":0}])
            }
        }
        let (result, cwd) = build(doc);
        assert_ne!(result["status"], "completed", "{result}");
        if variant < 3 {
            assert_eq!(result["code"], "CAD_IR_INVALID", "{result}");
            assert!(
                result["detail"]
                    .as_str()
                    .unwrap()
                    .contains(["BrokenReference", "DuplicateId", "InvalidValue"][variant]),
                "{result}"
            );
        } else {
            assert_eq!(result["code"], "SKETCH_CONSTRAINT_CONFLICT", "{result}");
        }
        assert!(!cwd.path().join("model.step").exists());
        assert!(!cwd.path().join("preview.glb").exists());
    }
}

#[test]
fn diagnostics_and_commands_respect_the_same_bindings() {
    use forma_core::{
        cad_ir::{apply_commands, Command, Document, ErrorCode},
        sketch_analysis::analyze_operation,
    };
    let original: Document = serde_json::from_value(fixture()).unwrap();
    let mut changed = original.clone();
    changed.parameters[0].value_mm = 80.0;
    let report = analyze_operation(
        changed.features[0].operation.clone(),
        changed.parameters.clone(),
        "base_sketch",
    )
    .unwrap();
    assert_eq!(report.status, "solved");
    assert_eq!(report.degrees_of_freedom, Some(0));
    assert_eq!(report.solved_points[0].x_mm, -40.0);
    assert_eq!(report.constraints.len(), 8);
    let command = Command::SetSketchPoint {
        feature_id: "base_sketch".into(),
        point_id: "a".into(),
        x_mm: 0.0,
        y_mm: 0.0,
    };
    let error = apply_commands(&original, "draft", "next", &[command]).unwrap_err();
    assert_eq!(error.code, ErrorCode::ParameterBound);
    assert_eq!(original.revision_id, "draft");
    let command = Command::SetParameter {
        parameter_id: "width".into(),
        value_mm: 80.0,
    };
    let next = apply_commands(&original, "draft", "next", &[command]).unwrap();
    assert_eq!(next.parameters[0].value_mm, 80.0);
    assert_eq!(original.parameters[0].value_mm, 40.0);
}

#[test]
fn typed_batch_unlinks_profiles_without_geometry_jumps() {
    use forma_core::cad_ir::{apply_commands, Command, Document, Operation};
    let original: Document = serde_json::from_value(fixture()).unwrap();
    let mut commands: Vec<Command> = [80.0, 30.0, 15.0, 12.0]
        .iter()
        .zip(&original.parameters)
        .map(|(value, parameter)| Command::SetParameter {
            parameter_id: parameter.id.clone(),
            value_mm: *value,
        })
        .collect();
    for feature in &original.features {
        if let Operation::Sketch2d { bindings, .. } = &feature.operation {
            for binding in bindings {
                commands.push(Command::RemoveSketchBinding {
                    feature_id: feature.id.clone(),
                    binding_id: binding.id().into(),
                });
            }
        }
    }
    let unlinked = apply_commands(&original, "draft", "unlinked", &commands).unwrap();
    let (result, _cwd) = build(serde_json::to_value(&unlinked).unwrap());
    assert_eq!(result["status"], "completed", "{result}");
    assert!(
        (result["volumeMm3"].as_f64().unwrap() - 50400.0).abs() < 0.001,
        "{result}"
    );
    assert_eq!(result["boundsMm"], serde_json::json!([80.0, 30.0, 27.0]));
    let later = apply_commands(
        &unlinked,
        "unlinked",
        "later",
        &[Command::SetParameter {
            parameter_id: "triangle_height".into(),
            value_mm: 20.0,
        }],
    )
    .unwrap();
    let (later_result, _cwd) = build(serde_json::to_value(later).unwrap());
    assert_eq!(later_result["status"], "completed", "{later_result}");
    assert_eq!(later_result["boundsMm"], result["boundsMm"]);
    assert!(
        (later_result["volumeMm3"].as_f64().unwrap() - result["volumeMm3"].as_f64().unwrap()).abs()
            < 0.001
    );
    assert_eq!(original.parameters[0].value_mm, 40.0);
}
