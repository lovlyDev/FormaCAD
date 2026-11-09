use super::super::*;
use serde_json::Value;
use std::collections::HashMap;

pub(super) fn apply(document: &mut Document, command: &Command) -> Result<()> {
    let Command::DuplicateBody {
        body_id,
        new_body_id,
        name,
        offset_mm,
    } = command
    else {
        return Err(ValidationError::new(ErrorCode::UnsupportedField, None));
    };
    let invalid = |code| ValidationError::new(code, Some(new_body_id.clone()));
    if !super::valid_id(new_body_id) || new_body_id.len() > 64 {
        return Err(invalid(ErrorCode::InvalidId));
    }
    let original = document
        .bodies
        .iter()
        .find(|body| body.id == *body_id)
        .ok_or_else(|| ValidationError::new(ErrorCode::MissingTarget, Some(body_id.clone())))?;
    let required = required_features(document, &original.source_feature_id);
    let prefix = new_body_id;
    let features: Vec<_> = document
        .features
        .iter()
        .filter(|feature| required.contains(feature.id.as_str()))
        .collect();
    let feature_ids: HashMap<_, _> = features
        .iter()
        .enumerate()
        .map(|(index, feature)| (feature.id.as_str(), format!("{prefix}_f{index}")))
        .collect();
    let mut copies = Vec::with_capacity(features.len() + 1);
    let mut parameter_ids = HashMap::new();
    for feature in features {
        let mut operation = serde_json::to_value(&feature.operation)
            .map_err(|_| invalid(ErrorCode::InvalidDocument))?;
        remap_operation(&mut operation, &feature_ids, &mut parameter_ids, prefix)?;
        let operation =
            serde_json::from_value(operation).map_err(|_| invalid(ErrorCode::InvalidDocument))?;
        copies.push(Feature {
            id: feature_ids[feature.id.as_str()].clone(),
            name: feature.name.clone(),
            operation,
            suppressed: feature.suppressed,
        });
    }
    let mut source = feature_ids
        .get(original.source_feature_id.as_str())
        .cloned()
        .ok_or_else(|| invalid(ErrorCode::BrokenReference))?;
    if *offset_mm != [0.; 3] {
        let id = format!("{prefix}_placement");
        copies.push(Feature {
            id: id.clone(),
            name: name.clone(),
            operation: Operation::Translate {
                body_feature_id: source,
                offset_mm: *offset_mm,
            },
            suppressed: false,
        });
        source = id;
    }
    let copied_parameters = document
        .parameters
        .iter()
        .filter_map(|value| {
            parameter_ids.get(&value.id).map(|id| Parameter {
                id: id.clone(),
                name: value.name.clone(),
                value_mm: value.value_mm,
            })
        })
        .collect::<Vec<_>>();
    document.parameters.extend(copied_parameters);
    document.features.extend(copies);
    document.bodies.push(Body {
        id: new_body_id.clone(),
        name: name.clone(),
        source_feature_id: source,
    });
    Ok(())
}

// Only schema-wide references change. Sketch-local ids and immutable STEP seals stay intact.
fn remap_operation(
    value: &mut Value,
    features: &HashMap<&str, String>,
    parameter_ids: &mut HashMap<String, String>,
    prefix: &str,
) -> Result<()> {
    let feature_id = |value: &mut Value| {
        let id = value
            .as_str()
            .ok_or_else(|| ValidationError::new(ErrorCode::InvalidDocument, None))?;
        let cloned = features
            .get(id)
            .ok_or_else(|| ValidationError::new(ErrorCode::BrokenReference, Some(id.to_owned())))?;
        *value = Value::String(cloned.clone());
        Ok(())
    };
    match value {
        Value::Object(fields) => {
            for (key, field) in fields {
                match key.as_str() {
                    "bodyFeatureId" | "leftFeatureId" | "rightFeatureId" | "sketchId"
                    | "ownerFeatureId" | "featureId" => feature_id(field)?,
                    "occurrencePath" => {
                        let path = field.as_array_mut().ok_or_else(|| {
                            ValidationError::new(ErrorCode::InvalidDocument, None)
                        })?;
                        for entry in path {
                            feature_id(entry)?;
                        }
                    }
                    "parameterId" => {
                        let id = field.as_str().ok_or_else(|| {
                            ValidationError::new(ErrorCode::InvalidDocument, None)
                        })?;
                        let index = parameter_ids.len();
                        let copied = parameter_ids
                            .entry(id.to_owned())
                            .or_insert_with(|| format!("{prefix}_p{index}"));
                        *field = Value::String(copied.clone());
                    }
                    _ => remap_operation(field, features, parameter_ids, prefix)?,
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                remap_operation(item, features, parameter_ids, prefix)?;
            }
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn duplicate_rebinds_features_parameters_and_semantic_occurrences() {
        let original: Document = serde_json::from_value(json!({
            "schemaVersion":2,"revisionId":"baseline",
            "parameters":[{"id":"width","name":"Width","valueMm":20.}],
            "features":[
                {"id":"profile","name":"Profile","operation":{"type":"rectangle",
                    "width":{"kind":"parameter","parameterId":"width"},
                    "depth":{"kind":"literal","mm":10.}}},
                {"id":"pad","name":"Pad","operation":{"type":"extrude","sketchId":"profile",
                    "distance":{"kind":"literal","mm":8.}}},
                {"id":"shift","name":"Shift","operation":{"type":"translate","bodyFeatureId":"pad",
                    "offsetMm":[2.,0.,0.]}},
                {"id":"cut","name":"Face cut","operation":{"type":"extrudeFace","bodyFeatureId":"shift",
                    "distance":{"kind":"literal","mm":-1.},"reference":{"schemaVersion":1,"kind":"face",
                    "ownerFeatureId":"pad","role":"box-face:zmax","occurrencePath":["shift"]}}}
            ],
            "bodies":[{"id":"part","name":"Part","sourceFeatureId":"cut"}]
        })).unwrap();
        let command = Command::DuplicateBody {
            body_id: "part".into(),
            new_body_id: "copy".into(),
            name: "Part copy".into(),
            offset_mm: [30., 0., 0.],
        };
        let duplicate = apply_commands(
            &original,
            "baseline",
            "next",
            std::slice::from_ref(&command),
        )
        .unwrap();
        assert_eq!(&duplicate.features[..4], &original.features);
        assert_eq!(duplicate.bodies[1].source_feature_id, "copy_placement");
        assert_eq!(duplicate.parameters[1].id, "copy_p0");
        let Operation::ExtrudeFace { reference, .. } = &duplicate.features[7].operation else {
            panic!("Expected copied face cut");
        };
        assert_eq!(reference.owner_feature_id, "copy_f1");
        assert_eq!(reference.occurrence_path, ["copy_f2"]);
        let changed = apply_commands(
            &duplicate,
            "next",
            "changed",
            &[Command::SetParameter {
                parameter_id: "copy_p0".into(),
                value_mm: 32.,
            }],
        )
        .unwrap();
        assert_eq!(changed.parameters[0].value_mm, 20.);
        assert_eq!(changed.parameters[1].value_mm, 32.);
        assert!(apply_commands(&duplicate, "next", "changed", &[command]).is_err());
    }
}
