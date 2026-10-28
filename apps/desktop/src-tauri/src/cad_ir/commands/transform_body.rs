use super::super::{
    affine_transform, Command, Document, ErrorCode, Feature, Operation, Result, ValidationError,
};

/// All target updates are staged in the document copy owned by apply_commands.
pub(super) fn apply(document: &mut Document, command: &Command) -> Result<()> {
    let Command::TransformBody {
        body_id,
        feature_id,
        name,
        matrix,
    } = command
    else {
        return Err(ValidationError::new(ErrorCode::UnsupportedField, None));
    };
    affine_transform::validate(matrix, body_id)?;
    let body = document
        .bodies
        .iter_mut()
        .find(|body| body.id == *body_id)
        .ok_or_else(|| ValidationError::new(ErrorCode::MissingTarget, Some(body_id.clone())))?;
    if affine_transform::is_identity(matrix) {
        return Ok(());
    }
    let source = body.source_feature_id.clone();
    document.features.push(Feature {
        id: feature_id.clone(),
        name: name.clone(),
        suppressed: false,
        operation: Operation::Transform {
            body_feature_id: source,
            matrix: *matrix,
        },
    });
    body.source_feature_id = feature_id.clone();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cad_ir::{apply_commands, Body, Dimension, SCHEMA_VERSION};
    fn document() -> Document {
        Document {
            schema_version: SCHEMA_VERSION,
            revision_id: "before".into(),
            parameters: vec![],
            features: vec![
                Feature {
                    id: "profile".into(),
                    name: "Profile".into(),
                    suppressed: false,
                    operation: Operation::Rectangle {
                        width: Dimension::Literal { mm: 20. },
                        depth: Dimension::Literal { mm: 10. },
                    },
                },
                Feature {
                    id: "pad".into(),
                    name: "Pad".into(),
                    suppressed: false,
                    operation: Operation::Extrude {
                        sketch_id: "profile".into(),
                        distance: Dimension::Literal { mm: 5. },
                    },
                },
            ],
            bodies: vec![Body {
                id: "body".into(),
                name: "Body".into(),
                source_feature_id: "pad".into(),
            }],
        }
    }
    fn command(x: f64) -> Command {
        Command::TransformBody {
            body_id: "body".into(),
            feature_id: "move".into(),
            name: "Move".into(),
            matrix: [1., 0., 0., x, 0., 1., 0., 0., 0., 0., 1., 0.],
        }
    }
    #[test]
    fn captured_transform_is_one_staged_command_and_retains_input_identity() {
        let before = document();
        let after = apply_commands(&before, "before", "after", &[command(10.)]).unwrap();
        assert_eq!(before.bodies[0].source_feature_id, "pad");
        assert_eq!(before.features.len(), 2);
        assert_eq!(after.bodies[0].source_feature_id, "move");
        assert_eq!(after.features[2].operation.dependencies(), vec!["pad"]);
        assert_eq!(
            after.features[2].operation.suppression_source(),
            Some("pad")
        );
        let serialized = serde_json::to_string(&after).unwrap();
        assert_eq!(
            serde_json::from_str::<Document>(&serialized).unwrap(),
            after
        );
    }
    #[test]
    fn invalid_second_command_rolls_back_the_entire_batch() {
        let before = document();
        let mut bad = command(0.);
        if let Command::TransformBody {
            matrix, feature_id, ..
        } = &mut bad
        {
            matrix[0] = 0.;
            *feature_id = "bad".into();
        }
        assert_eq!(
            apply_commands(&before, "before", "after", &[command(10.), bad])
                .unwrap_err()
                .code,
            ErrorCode::InvalidValue
        );
        assert_eq!(before, document());
        assert_eq!(
            apply_commands(&before, "stale", "after", &[command(10.)])
                .unwrap_err()
                .code,
            ErrorCode::RevisionConflict
        );
        assert_eq!(
            apply_commands(&before, "before", "after", &[command(0.)])
                .unwrap_err()
                .code,
            ErrorCode::NoChange
        );
    }
    #[test]
    fn matrix_bounds_match_native_and_reflection_is_separate() {
        for value in [0., -1., f64::NAN, f64::INFINITY, 1001.] {
            let mut edit = command(10.);
            if let Command::TransformBody { matrix, .. } = &mut edit {
                matrix[0] = value;
            }
            assert_eq!(
                apply_commands(&document(), "before", "after", &[edit])
                    .unwrap_err()
                    .code,
                ErrorCode::InvalidValue
            );
        }
        assert_eq!(
            apply_commands(&document(), "before", "after", &[command(10001.)])
                .unwrap_err()
                .code,
            ErrorCode::InvalidValue
        );
    }
}
