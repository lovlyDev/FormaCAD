use super::{ErrorCode, Result, ValidationError, MAX_DIMENSION_MM};

/// OCCT Precision::Confusion(), in millimetres.
const MIN_CLEARANCE_MM: f64 = 1e-7;

pub(super) fn validate_radii(major: f64, minor: f64, id: &str) -> Result<()> {
    if !major.is_finite()
        || !minor.is_finite()
        || major > MAX_DIMENSION_MM
        || minor <= MIN_CLEARANCE_MM
        || major - minor <= MIN_CLEARANCE_MM
    {
        return Err(ValidationError::new(
            ErrorCode::InvalidValue,
            Some(id.to_owned()),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cad_ir::{apply_commands, Command, Document};
    use serde_json::json;

    #[test]
    fn invalid_torus_rejects_the_whole_command_batch() {
        let current: Document = serde_json::from_value(json!({
            "schemaVersion": 2,
            "revisionId": "initial",
            "parameters": [],
            "features": [],
            "bodies": []
        }))
        .unwrap();
        let commands = |minor_radius| {
            serde_json::from_value::<Vec<Command>>(json!([
                {
                    "command": "add_feature",
                    "feature": {
                        "id": "torus",
                        "name": "Torus",
                        "operation": {
                            "type": "torus",
                            "majorRadius": {"kind": "literal", "mm": 10},
                            "minorRadius": {"kind": "literal", "mm": minor_radius}
                        }
                    }
                },
                {
                    "command": "add_body",
                    "body": {"id": "ring", "name": "Ring", "sourceFeatureId": "torus"}
                }
            ]))
            .unwrap()
        };
        let before = current.clone();
        let error = apply_commands(&current, "initial", "next", &commands(10)).unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidValue);
        assert_eq!(current, before);
        let next = apply_commands(&current, "initial", "next", &commands(3)).unwrap();
        assert_eq!(next.bodies[0].source_feature_id, "torus");
        assert_eq!(next.revision_id, "next");
    }
}
