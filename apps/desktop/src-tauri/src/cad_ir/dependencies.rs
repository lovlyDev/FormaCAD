//! Ordered feature dependencies shared by regeneration and history inspection.
use super::{Document, Operation};
use std::collections::HashSet;

impl Operation {
    pub fn dependencies(&self) -> Vec<&str> {
        match self {
            Self::Extrude { sketch_id, .. } | Self::Revolve { sketch_id, .. } => vec![sketch_id],
            Self::Hole {
                body_feature_id, ..
            }
            | Self::Fillet {
                body_feature_id, ..
            }
            | Self::FilletEdge {
                body_feature_id, ..
            }
            | Self::FilletReferencedEdge {
                body_feature_id, ..
            }
            | Self::Chamfer {
                body_feature_id, ..
            }
            | Self::Translate {
                body_feature_id, ..
            }
            | Self::Rotate {
                body_feature_id, ..
            }
            | Self::Mirror {
                body_feature_id, ..
            } => vec![body_feature_id],
            Self::Boolean {
                left_feature_id,
                right_feature_id,
                ..
            } => {
                vec![left_feature_id, right_feature_id]
            }
            Self::ImportStep { .. }
            | Self::Sketch2d { .. }
            | Self::Rectangle { .. }
            | Self::Circle { .. }
            | Self::Sphere { .. }
            | Self::Cone { .. } => vec![],
        }
    }

    /// Suppression of a modifier passes through its unmodified input body.
    /// Generators and multi-input booleans cannot be suppressed safely.
    pub fn suppression_source(&self) -> Option<&str> {
        match self {
            Self::Hole {
                body_feature_id, ..
            }
            | Self::Fillet {
                body_feature_id, ..
            }
            | Self::FilletEdge {
                body_feature_id, ..
            }
            | Self::FilletReferencedEdge {
                body_feature_id, ..
            }
            | Self::Chamfer {
                body_feature_id, ..
            }
            | Self::Translate {
                body_feature_id, ..
            }
            | Self::Rotate {
                body_feature_id, ..
            }
            | Self::Mirror {
                body_feature_id, ..
            } => Some(body_feature_id),
            _ => None,
        }
    }
}

/// A validated document has only backward references, so one reverse pass
/// finds the entire chain needed to build an earlier body output.
pub fn required_features<'a>(document: &'a Document, output_id: &'a str) -> HashSet<&'a str> {
    let mut required = HashSet::from([output_id]);
    for feature in document.features.iter().rev() {
        if required.contains(feature.id.as_str()) {
            required.extend(feature.operation.dependencies());
        }
    }
    required
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cad_ir::{Body, Dimension, Feature, SCHEMA_VERSION};

    #[test]
    fn rollback_ignores_later_independent_features() {
        let sketch = Feature {
            id: "sketch".into(),
            name: "Sketch".into(),
            suppressed: false,
            operation: Operation::Circle {
                radius: Dimension::Literal { mm: 2.0 },
            },
        };
        let pad = Feature {
            id: "pad".into(),
            name: "Pad".into(),
            suppressed: false,
            operation: Operation::Extrude {
                sketch_id: "sketch".into(),
                distance: Dimension::Literal { mm: 5.0 },
            },
        };
        let later = Feature {
            id: "later".into(),
            name: "Later".into(),
            suppressed: false,
            operation: Operation::Fillet {
                body_feature_id: "pad".into(),
                radius: Dimension::Literal { mm: 0.5 },
            },
        };
        let document = Document {
            schema_version: SCHEMA_VERSION,
            revision_id: "revision".into(),
            parameters: vec![],
            features: vec![sketch, pad, later],
            bodies: vec![Body {
                id: "body".into(),
                name: "Body".into(),
                source_feature_id: "pad".into(),
            }],
        };
        document.validate().unwrap();
        assert_eq!(
            required_features(&document, "pad"),
            HashSet::from(["sketch", "pad"])
        );
    }
}
