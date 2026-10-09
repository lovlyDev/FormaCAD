use super::*;
use std::collections::HashMap;

fn resolved_dimension(
    value: &Dimension,
    parameters: &HashMap<&str, f64>,
) -> crate::core::Result<f64> {
    match value {
        Dimension::Literal { mm } => Ok(*mm),
        Dimension::Parameter { parameter_id } => parameters
            .get(parameter_id.as_str())
            .copied()
            .ok_or_else(|| {
                app_error(ValidationError::new(
                    ErrorCode::BrokenReference,
                    Some(parameter_id.clone()),
                ))
            }),
    }
}

impl Document {
    /// Temporary geometry adapter while the native kernel is being built. It
    /// retains semantic feature IDs and resolves parameters only at this edge.
    pub fn compile_legacy_geometry(
        &self,
    ) -> crate::core::Result<crate::cad_document::CompiledDocument> {
        use crate::cad_document::{
            BooleanMode as LegacyMode, CadDocument, Feature as LegacyFeature,
            Operation as LegacyOperation,
        };

        self.validate().map_err(app_error)?;
        if self.bodies.len() != 1 || self.features.is_empty() {
            return Err(app_error(ValidationError::new(
                ErrorCode::InvalidBody,
                None,
            )));
        }
        let parameters: HashMap<&str, f64> = self
            .parameters
            .iter()
            .map(|p| (p.id.as_str(), p.value_mm))
            .collect();
        let mut features = Vec::with_capacity(self.features.len());
        for feature in &self.features {
            let operation = match &feature.operation {
                Operation::Sketch2d { .. } => {
                    return Err(app_error(ValidationError::new(
                        ErrorCode::UnsupportedField,
                        Some(feature.id.clone()),
                    )));
                }
                Operation::Rectangle { width, depth } => LegacyOperation::Rectangle {
                    width: resolved_dimension(width, &parameters)?,
                    depth: resolved_dimension(depth, &parameters)?,
                },
                Operation::Circle { radius } => LegacyOperation::Circle {
                    radius: resolved_dimension(radius, &parameters)?,
                },
                Operation::ImportStep { .. }
                | Operation::Sphere { .. }
                | Operation::Cone { .. }
                | Operation::Hole { .. }
                | Operation::Fillet { .. }
                | Operation::FilletEdge { .. }
                | Operation::FilletReferencedEdge { .. }
                | Operation::Chamfer { .. }
                | Operation::Rotate { .. }
                | Operation::Mirror { .. }
                | Operation::Revolve { .. } => {
                    return Err(app_error(ValidationError::new(
                        ErrorCode::UnsupportedField,
                        Some(feature.id.clone()),
                    )));
                }
                Operation::Extrude {
                    sketch_id,
                    distance,
                } => LegacyOperation::Extrude {
                    sketch: sketch_id.clone(),
                    distance: resolved_dimension(distance, &parameters)?,
                },
                Operation::Translate {
                    body_feature_id,
                    offset_mm,
                } => LegacyOperation::Translate {
                    body: body_feature_id.clone(),
                    offset: *offset_mm,
                },
                Operation::Boolean {
                    left_feature_id,
                    right_feature_id,
                    mode,
                } => LegacyOperation::Boolean {
                    left: left_feature_id.clone(),
                    right: right_feature_id.clone(),
                    mode: match mode {
                        BooleanMode::Union => LegacyMode::Union,
                        BooleanMode::Cut => LegacyMode::Cut,
                        BooleanMode::Intersect => LegacyMode::Intersect,
                    },
                },
            };
            features.push(LegacyFeature {
                id: feature.id.clone(),
                operation,
            });
        }
        CadDocument {
            version: 1,
            features,
            output: self.bodies[0].source_feature_id.clone(),
        }
        .compile()
    }
}

/// Accept either the existing typed document or the new IR, with no raw-code fallback.
pub fn compile_json(source: &str) -> crate::core::Result<crate::cad_document::CompiledDocument> {
    let value: serde_json::Value = serde_json::from_str(source)?;
    match value.get("schemaVersion") {
        Some(serde_json::Value::Number(version))
            if version.as_u64() == Some(SCHEMA_VERSION as u64) =>
        {
            let document: Document = serde_json::from_value(value)?;
            document.compile_legacy_geometry()
        }
        Some(_) => Err(app_error(ValidationError::new(
            ErrorCode::InvalidDocument,
            None,
        ))),
        None => crate::cad_document::CadDocument::parse(source)?.compile(),
    }
}
