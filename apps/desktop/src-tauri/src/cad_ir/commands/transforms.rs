//! Editing a rigid modifier preserves its feature identity and body dependency.
use super::super::{Command, Document, ErrorCode, Operation, Result, ValidationError};

pub(super) fn apply(document: &mut Document, command: &Command) -> Result<()> {
    let id = match command {
        Command::SetRotation { feature_id, .. }
        | Command::SetMirrorPlane { feature_id, .. }
        | Command::SetRevolution { feature_id, .. } => feature_id,
        _ => return Err(ValidationError::new(ErrorCode::UnsupportedField, None)),
    };
    let feature = document
        .features
        .iter_mut()
        .find(|f| f.id == *id)
        .ok_or_else(|| ValidationError::new(ErrorCode::MissingTarget, Some(id.clone())))?;
    match (&mut feature.operation, command) {
        (
            Operation::Revolve {
                axis_origin_mm,
                axis_direction,
                angle_deg,
                ..
            },
            Command::SetRevolution {
                axis_origin_mm: origin,
                axis_direction: direction,
                angle_deg: angle,
                ..
            },
        ) => {
            *axis_origin_mm = *origin;
            *axis_direction = *direction;
            *angle_deg = *angle;
        }
        (
            Operation::Rotate {
                axis_origin_mm,
                axis_direction,
                angle_deg,
                ..
            },
            Command::SetRotation {
                axis_origin_mm: origin,
                axis_direction: direction,
                angle_deg: angle,
                ..
            },
        ) => {
            *axis_origin_mm = *origin;
            *axis_direction = *direction;
            *angle_deg = *angle;
        }
        (
            Operation::Mirror {
                plane_origin_mm,
                plane_normal,
                ..
            },
            Command::SetMirrorPlane {
                plane_origin_mm: origin,
                plane_normal: normal,
                ..
            },
        ) => {
            *plane_origin_mm = *origin;
            *plane_normal = *normal;
        }
        _ => {
            return Err(ValidationError::new(
                ErrorCode::UnsupportedField,
                Some(id.clone()),
            ))
        }
    }
    Ok(())
}
