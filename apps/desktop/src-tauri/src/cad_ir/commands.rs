mod bindings;
#[cfg(test)]
mod bindings_tests;
mod sketch;
mod topology_refs;
mod transforms;
use super::validation::valid_id;
use super::*;

/// Stage a complete command batch. The caller must validate its BREP result and
/// persist both the new document and geometry before exposing the revision.
pub fn apply_commands(
    current: &Document,
    expected_revision: &str,
    next_revision: &str,
    commands: &[Command],
) -> Result<Document> {
    current.validate()?;
    if current.revision_id != expected_revision {
        return Err(ValidationError::new(ErrorCode::RevisionConflict, None));
    }
    if !valid_id(next_revision) || next_revision == expected_revision {
        return Err(ValidationError::new(ErrorCode::InvalidId, None));
    }
    if commands.is_empty() {
        return Err(ValidationError::new(ErrorCode::NoCommands, None));
    }
    if commands.len() > 256 {
        return Err(ValidationError::new(ErrorCode::InvalidDocument, None));
    }
    let mut staged = current.clone();
    for command in commands {
        match command {
            Command::SetTopologyReference { .. } => topology_refs::apply(&mut staged, command)?,
            Command::AddParameter { parameter } => staged.parameters.push(parameter.clone()),
            Command::AddFeature { feature } => staged.features.push(feature.clone()),
            Command::AddBody { body } => staged.bodies.push(body.clone()),
            Command::SetBodySource {
                body_id,
                source_feature_id,
            } => {
                let body = staged
                    .bodies
                    .iter_mut()
                    .find(|body| body.id == *body_id)
                    .ok_or_else(|| {
                        ValidationError::new(ErrorCode::MissingTarget, Some(body_id.clone()))
                    })?;
                body.source_feature_id = source_feature_id.clone();
            }
            Command::SetParameter {
                parameter_id,
                value_mm,
            } => {
                let parameter = staged
                    .parameters
                    .iter_mut()
                    .find(|parameter| parameter.id == *parameter_id)
                    .ok_or_else(|| {
                        ValidationError::new(ErrorCode::MissingTarget, Some(parameter_id.clone()))
                    })?;
                parameter.value_mm = *value_mm;
            }
            Command::SetLiteral {
                feature_id,
                field,
                value_mm,
            } => {
                let feature = staged
                    .features
                    .iter_mut()
                    .find(|feature| feature.id == *feature_id)
                    .ok_or_else(|| {
                        ValidationError::new(ErrorCode::MissingTarget, Some(feature_id.clone()))
                    })?;
                let dimension = match (&mut feature.operation, field) {
                    (Operation::Rectangle { width, .. }, DimensionField::Width) => width,
                    (Operation::Rectangle { depth, .. }, DimensionField::Depth) => depth,
                    (Operation::Circle { radius }, DimensionField::Radius) => radius,
                    (Operation::Sphere { radius }, DimensionField::Radius) => radius,
                    (Operation::Cone { bottom_radius, .. }, DimensionField::BottomRadius) => {
                        bottom_radius
                    }
                    (Operation::Cone { top_radius, .. }, DimensionField::TopRadius) => top_radius,
                    (Operation::Cone { height, .. }, DimensionField::Height) => height,
                    (Operation::Extrude { distance, .. }, DimensionField::Distance) => distance,
                    (Operation::Hole { radius, .. }, DimensionField::Radius) => radius,
                    (Operation::Hole { depth, .. }, DimensionField::HoleDepth) => depth,
                    (Operation::Fillet { radius, .. }, DimensionField::Radius) => radius,
                    (Operation::FilletEdge { radius, .. }, DimensionField::Radius) => radius,
                    (Operation::FilletReferencedEdge { radius, .. }, DimensionField::Radius) => {
                        radius
                    }
                    (Operation::Chamfer { distance, .. }, DimensionField::Distance) => distance,
                    _ => {
                        return Err(ValidationError::new(
                            ErrorCode::UnsupportedField,
                            Some(feature_id.clone()),
                        ))
                    }
                };
                match dimension {
                    Dimension::Literal { mm } => *mm = *value_mm,
                    Dimension::Parameter { .. } => {
                        return Err(ValidationError::new(
                            ErrorCode::ParameterBound,
                            Some(feature_id.clone()),
                        ))
                    }
                }
            }
            Command::RenameFeature { feature_id, name } => {
                let feature = staged
                    .features
                    .iter_mut()
                    .find(|feature| feature.id == *feature_id)
                    .ok_or_else(|| {
                        ValidationError::new(ErrorCode::MissingTarget, Some(feature_id.clone()))
                    })?;
                feature.name = name.clone();
            }
            Command::SetFeatureSuppressed {
                feature_id,
                suppressed,
            } => {
                let feature = staged
                    .features
                    .iter_mut()
                    .find(|feature| feature.id == *feature_id)
                    .ok_or_else(|| {
                        ValidationError::new(ErrorCode::MissingTarget, Some(feature_id.clone()))
                    })?;
                feature.suppressed = *suppressed;
            }
            Command::SetRotation { .. }
            | Command::SetMirrorPlane { .. }
            | Command::SetRevolution { .. } => transforms::apply(&mut staged, command)?,
            Command::SetSketchPoint { .. }
            | Command::SetSketchPlane { .. }
            | Command::SetSketchLength { .. }
            | Command::AddSketchConstraint { .. }
            | Command::RemoveSketchConstraint { .. }
            | Command::AddSketchBinding { .. }
            | Command::SetSketchBinding { .. }
            | Command::RemoveSketchBinding { .. } => sketch::apply(&mut staged, command)?,
        }
    }
    staged.validate()?;
    if staged.features == current.features
        && staged.parameters == current.parameters
        && staged.bodies == current.bodies
    {
        return Err(ValidationError::new(ErrorCode::NoChange, None));
    }
    staged.revision_id = next_revision.to_owned();
    Ok(staged)
}
