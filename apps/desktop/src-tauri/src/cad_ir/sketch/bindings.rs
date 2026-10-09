//! Explicit affine parameter links, shared by validation, diagnostics and BREP execution.
use super::*;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Axis {
    X,
    Y,
    Z,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ParameterCoordinate {
    pub kind: ParameterKind,
    pub parameter_id: String,
    pub scale: f64,
    pub offset_mm: f64,
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ParameterKind {
    Parameter,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "target",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum CoordinateBinding {
    Point {
        id: String,
        point_id: String,
        axis: Axis,
        value: ParameterCoordinate,
    },
    Origin {
        id: String,
        axis: Axis,
        value: ParameterCoordinate,
    },
}
impl CoordinateBinding {
    pub fn id(&self) -> &str {
        match self {
            Self::Point { id, .. } | Self::Origin { id, .. } => id,
        }
    }
    fn value(&self) -> &ParameterCoordinate {
        match self {
            Self::Point { value, .. } | Self::Origin { value, .. } => value,
        }
    }
}
fn resolve_value(
    value: &ParameterCoordinate,
    parameters: &HashMap<&str, f64>,
    feature: &str,
) -> Result<f64, ValidationError> {
    let parameter = parameters
        .get(value.parameter_id.as_str())
        .ok_or_else(|| error(ErrorCode::BrokenReference, feature))?;
    let coordinate = *parameter * value.scale + value.offset_mm;
    if !valid_id(&value.parameter_id)
        || !value.scale.is_finite()
        || value.scale.abs() > 10000.0
        || !value.offset_mm.is_finite()
        || value.offset_mm.abs() > 10000.0
        || !coordinate.is_finite()
        || coordinate.abs() > 10000.0
    {
        return Err(error(ErrorCode::InvalidValue, feature));
    }
    Ok(coordinate)
}
pub fn resolve(
    origin: [f64; 3],
    points: &[SketchPoint],
    constraints: &[SketchConstraint],
    bindings: &[CoordinateBinding],
    parameters: &HashMap<&str, f64>,
    feature: &str,
) -> Result<([f64; 3], Vec<SketchPoint>), ValidationError> {
    if bindings.len() + constraints.len() > 64 {
        return Err(error(ErrorCode::InvalidSketch, feature));
    }
    let mut origin = origin;
    let mut points = points.to_vec();
    let mut ids: HashSet<_> = constraints.iter().map(SketchConstraint::id).collect();
    let mut targets = HashSet::new();
    for binding in bindings {
        if !valid_id(binding.id()) || !ids.insert(binding.id()) {
            return Err(error(ErrorCode::DuplicateId, feature));
        }
        let coordinate = resolve_value(binding.value(), parameters, feature)?;
        match binding {
            CoordinateBinding::Point { point_id, axis, .. } => {
                if matches!(axis, Axis::Z) || !targets.insert((point_id.as_str(), *axis as u8)) {
                    return Err(error(ErrorCode::InvalidSketch, feature));
                }
                let point = points
                    .iter_mut()
                    .find(|point| point.id == *point_id)
                    .ok_or_else(|| error(ErrorCode::BrokenReference, feature))?;
                if matches!(axis, Axis::X) {
                    point.x_mm = coordinate;
                } else {
                    point.y_mm = coordinate;
                }
            }
            CoordinateBinding::Origin { axis, .. } => {
                if !targets.insert(("", *axis as u8)) {
                    return Err(error(ErrorCode::InvalidSketch, feature));
                }
                origin[*axis as usize] = coordinate;
            }
        }
    }
    Ok((origin, points))
}
pub(super) fn append<'a>(
    system: &mut system::ConstraintSystem<'a>,
    bindings: &'a [CoordinateBinding],
    parameters: &HashMap<&str, f64>,
    feature: &str,
) -> Result<(), ValidationError> {
    for binding in bindings {
        if let CoordinateBinding::Point {
            id, point_id, axis, ..
        } = binding
        {
            let position = system
                .positions
                .get(point_id.as_str())
                .ok_or_else(|| error(ErrorCode::BrokenReference, feature))?;
            let index = system.equations.len();
            system.equations.push(Equation::Fixed(
                *position + usize::from(matches!(axis, Axis::Y)),
                resolve_value(binding.value(), parameters, feature)?,
            ));
            system
                .constraint_ranges
                .push((id.as_str(), index..index + 1));
        }
    }
    Ok(())
}
