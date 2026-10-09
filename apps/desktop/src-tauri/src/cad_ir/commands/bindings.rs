//! Typed coordinate-link edits. Removing one link materializes only its axis.
use super::super::*;
use crate::cad_ir::sketch::bindings::{Axis, CoordinateBinding};
use std::collections::HashMap;
fn operation<'a>(features: &'a mut [Feature], feature_id: &str) -> Result<&'a mut Operation> {
    let feature = features
        .iter_mut()
        .find(|feature| feature.id == feature_id)
        .ok_or_else(|| {
            ValidationError::new(ErrorCode::MissingTarget, Some(feature_id.to_owned()))
        })?;
    if !matches!(feature.operation, Operation::Sketch2d { .. }) {
        return Err(ValidationError::new(
            ErrorCode::UnsupportedField,
            Some(feature_id.to_owned()),
        ));
    }
    Ok(&mut feature.operation)
}
pub(super) fn add(
    document: &mut Document,
    feature_id: &str,
    binding: &CoordinateBinding,
) -> Result<()> {
    let Operation::Sketch2d { bindings, .. } = operation(&mut document.features, feature_id)?
    else {
        unreachable!()
    };
    bindings.push(binding.clone());
    Ok(())
}
pub(super) fn set(
    document: &mut Document,
    feature_id: &str,
    binding: &CoordinateBinding,
) -> Result<()> {
    let Operation::Sketch2d { bindings, .. } = operation(&mut document.features, feature_id)?
    else {
        unreachable!()
    };
    let current = bindings
        .iter_mut()
        .find(|current| current.id() == binding.id())
        .ok_or_else(|| {
            ValidationError::new(ErrorCode::MissingTarget, Some(binding.id().to_owned()))
        })?;
    *current = binding.clone();
    Ok(())
}
pub(super) fn remove(document: &mut Document, feature_id: &str, binding_id: &str) -> Result<()> {
    let parameters: HashMap<_, _> = document
        .parameters
        .iter()
        .map(|parameter| (parameter.id.as_str(), parameter.value_mm))
        .collect();
    let Operation::Sketch2d {
        origin_mm,
        points,
        constraints,
        bindings,
        ..
    } = operation(&mut document.features, feature_id)?
    else {
        unreachable!()
    };
    let index = bindings
        .iter()
        .position(|binding| binding.id() == binding_id)
        .ok_or_else(|| {
            ValidationError::new(ErrorCode::MissingTarget, Some(binding_id.to_owned()))
        })?;
    let (resolved_origin, resolved_points) = crate::cad_ir::sketch::bindings::resolve(
        *origin_mm,
        points,
        constraints,
        bindings,
        &parameters,
        feature_id,
    )?;
    match &bindings[index] {
        CoordinateBinding::Point { point_id, axis, .. } => {
            let target = points
                .iter_mut()
                .find(|point| &point.id == point_id)
                .ok_or_else(|| {
                    ValidationError::new(ErrorCode::BrokenReference, Some(point_id.clone()))
                })?;
            let effective = resolved_points
                .iter()
                .find(|point| &point.id == point_id)
                .ok_or_else(|| {
                    ValidationError::new(ErrorCode::BrokenReference, Some(point_id.clone()))
                })?;
            match axis {
                Axis::X => target.x_mm = effective.x_mm,
                Axis::Y => target.y_mm = effective.y_mm,
                Axis::Z => {
                    return Err(ValidationError::new(
                        ErrorCode::InvalidSketch,
                        Some(feature_id.to_owned()),
                    ))
                }
            }
        }
        CoordinateBinding::Origin { axis, .. } => {
            origin_mm[*axis as usize] = resolved_origin[*axis as usize]
        }
    }
    bindings.remove(index);
    Ok(())
}
