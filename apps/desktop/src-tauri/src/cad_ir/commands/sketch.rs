//! Editing commands for a bounded sketch, independent of generic feature edits.
use super::super::*;
use super::bindings;
pub(super) fn apply(document: &mut Document, command: &Command) -> Result<()> {
    match command {
        Command::SetSketchPoint {
            feature_id,
            point_id,
            x_mm,
            y_mm,
        } => {
            let feature = document
                .features
                .iter_mut()
                .find(|feature| feature.id == *feature_id)
                .ok_or_else(|| {
                    ValidationError::new(ErrorCode::MissingTarget, Some(feature_id.clone()))
                })?;
            let Operation::Sketch2d {
                points, bindings, ..
            } = &mut feature.operation
            else {
                return Err(ValidationError::new(
                    ErrorCode::UnsupportedField,
                    Some(feature_id.clone()),
                ));
            };
            if bindings.iter().any(|b|matches!(b,sketch::bindings::CoordinateBinding::Point{point_id:bound,..} if bound==point_id)) {
                    return Err(ValidationError::new(ErrorCode::ParameterBound,Some(feature_id.clone())));
                }
            let point = points
                .iter_mut()
                .find(|point| point.id == *point_id)
                .ok_or_else(|| {
                    ValidationError::new(ErrorCode::MissingTarget, Some(point_id.clone()))
                })?;
            point.x_mm = *x_mm;
            point.y_mm = *y_mm;
        }
        Command::SetSketchPlane {
            feature_id,
            plane,
            origin_mm,
        } => {
            let feature = document
                .features
                .iter_mut()
                .find(|feature| feature.id == *feature_id)
                .ok_or_else(|| {
                    ValidationError::new(ErrorCode::MissingTarget, Some(feature_id.clone()))
                })?;
            let Operation::Sketch2d {
                plane: current,
                origin_mm: origin,
                bindings,
                ..
            } = &mut feature.operation
            else {
                return Err(ValidationError::new(
                    ErrorCode::UnsupportedField,
                    Some(feature_id.clone()),
                ));
            };
            if bindings.iter().any(|b|matches!(b,sketch::bindings::CoordinateBinding::Origin{axis,..} if origin[*axis as usize]!=origin_mm[*axis as usize])) {
                    return Err(ValidationError::new(ErrorCode::ParameterBound,Some(feature_id.clone())));
                }
            *current = *plane;
            *origin = *origin_mm;
        }
        Command::SetSketchLength {
            feature_id,
            constraint_id,
            distance_mm,
        } => {
            let feature = document
                .features
                .iter_mut()
                .find(|feature| feature.id == *feature_id)
                .ok_or_else(|| {
                    ValidationError::new(ErrorCode::MissingTarget, Some(feature_id.clone()))
                })?;
            let Operation::Sketch2d { constraints, .. } = &mut feature.operation else {
                return Err(ValidationError::new(
                    ErrorCode::UnsupportedField,
                    Some(feature_id.clone()),
                ));
            };
            let constraint = constraints
                .iter_mut()
                .find(|constraint| constraint.id() == constraint_id)
                .ok_or_else(|| {
                    ValidationError::new(ErrorCode::MissingTarget, Some(constraint_id.clone()))
                })?;
            let SketchConstraint::Length { distance, .. } = constraint else {
                return Err(ValidationError::new(
                    ErrorCode::UnsupportedField,
                    Some(constraint_id.clone()),
                ));
            };
            match distance {
                Dimension::Literal { mm } => *mm = *distance_mm,
                Dimension::Parameter { .. } => {
                    return Err(ValidationError::new(
                        ErrorCode::ParameterBound,
                        Some(constraint_id.clone()),
                    ))
                }
            }
        }
        Command::AddSketchConstraint {
            feature_id,
            constraint,
        } => {
            let feature = document
                .features
                .iter_mut()
                .find(|feature| feature.id == *feature_id)
                .ok_or_else(|| {
                    ValidationError::new(ErrorCode::MissingTarget, Some(feature_id.clone()))
                })?;
            let Operation::Sketch2d { constraints, .. } = &mut feature.operation else {
                return Err(ValidationError::new(
                    ErrorCode::UnsupportedField,
                    Some(feature_id.clone()),
                ));
            };
            constraints.push(constraint.clone());
        }
        Command::RemoveSketchConstraint {
            feature_id,
            constraint_id,
        } => {
            let feature = document
                .features
                .iter_mut()
                .find(|feature| feature.id == *feature_id)
                .ok_or_else(|| {
                    ValidationError::new(ErrorCode::MissingTarget, Some(feature_id.clone()))
                })?;
            let Operation::Sketch2d { constraints, .. } = &mut feature.operation else {
                return Err(ValidationError::new(
                    ErrorCode::UnsupportedField,
                    Some(feature_id.clone()),
                ));
            };
            let previous = constraints.len();
            constraints.retain(|constraint| constraint.id() != constraint_id);
            if constraints.len() == previous {
                return Err(ValidationError::new(
                    ErrorCode::MissingTarget,
                    Some(constraint_id.clone()),
                ));
            }
        }
        Command::AddSketchBinding {
            feature_id,
            binding,
        } => bindings::add(document, feature_id, binding)?,
        Command::SetSketchBinding {
            feature_id,
            binding,
        } => bindings::set(document, feature_id, binding)?,
        Command::RemoveSketchBinding {
            feature_id,
            binding_id,
        } => bindings::remove(document, feature_id, binding_id)?,
        _ => return Err(ValidationError::new(ErrorCode::UnsupportedField, None)),
    }
    Ok(())
}
