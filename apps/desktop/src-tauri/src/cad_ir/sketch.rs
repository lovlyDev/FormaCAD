//! Bounded 2D sketch solving. Geometry lives in local workplane coordinates;
//! the CAD kernel owns the resulting BREP extrusion.
use super::{
    Dimension, ErrorCode, SketchConstraint, SketchLine, SketchPlane, SketchPoint, ValidationError,
};
use std::collections::{HashMap, HashSet};

const MAX_POINTS: usize = 32;
const MAX_LINES: usize = 64;
const MAX_CONSTRAINTS: usize = 64;
const TOLERANCE: f64 = 1e-6;

#[derive(Debug, Clone)]
pub struct SolvedSketch {
    pub plane: SketchPlane,
    pub origin_mm: [f64; 3],
    /// Closed outline in winding order, without repeating the first vertex.
    pub outline_xy: Vec<f64>,
    pub holes_xy: Vec<Vec<f64>>,
}

impl SketchConstraint {
    pub fn id(&self) -> &str {
        match self {
            Self::Fixed { id, .. }
            | Self::Horizontal { id, .. }
            | Self::Vertical { id, .. }
            | Self::Length { id, .. }
            | Self::Coincident { id, .. } => id,
        }
    }
}

fn error(code: ErrorCode, feature_id: &str) -> ValidationError {
    ValidationError::new(code, Some(feature_id.to_owned()))
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 80
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}

fn dimension(
    value: &Dimension,
    parameters: &HashMap<&str, f64>,
    feature_id: &str,
) -> Result<f64, ValidationError> {
    let mm = match value {
        Dimension::Literal { mm } => *mm,
        Dimension::Parameter { parameter_id } => *parameters
            .get(parameter_id.as_str())
            .ok_or_else(|| error(ErrorCode::BrokenReference, feature_id))?,
    };
    if !mm.is_finite() || mm <= 0.0 || mm > 10000.0 {
        return Err(error(ErrorCode::InvalidValue, feature_id));
    }
    Ok(mm)
}

pub mod analysis;
pub mod bindings;
mod outline;
mod profile;
mod solver;
mod structure;
mod system;
#[cfg(test)]
mod tests;

use profile::profile_loops;
use solver::{solve_constraints, Equation};
pub use structure::validate_structure;

/// Solve a bounded outer profile with optional holes. Every point/line/constraint keeps its own stable ID.
pub fn solve(
    plane: SketchPlane,
    origin_mm: [f64; 3],
    points: &[SketchPoint],
    lines: &[SketchLine],
    constraints: &[SketchConstraint],
    parameters: &HashMap<&str, f64>,
    feature_id: &str,
) -> Result<SolvedSketch, ValidationError> {
    validate_structure(
        origin_mm,
        points,
        lines,
        constraints,
        parameters,
        feature_id,
    )?;
    let mut system = system::prepare(points, lines, constraints, parameters, feature_id)?;
    solve_constraints(&mut system.variables, &system.equations, feature_id)?;
    solved_profile(&system, plane, origin_mm, lines, feature_id)
}

// Explicit sketch geometry and parameter context are passed together at the IR boundary.
#[allow(clippy::too_many_arguments)]
pub fn solve_bound(
    plane: SketchPlane,
    origin: [f64; 3],
    points: &[SketchPoint],
    lines: &[SketchLine],
    constraints: &[SketchConstraint],
    links: &[bindings::CoordinateBinding],
    parameters: &HashMap<&str, f64>,
    feature: &str,
) -> Result<SolvedSketch, ValidationError> {
    let (origin, points) =
        bindings::resolve(origin, points, constraints, links, parameters, feature)?;
    validate_structure(origin, &points, lines, constraints, parameters, feature)?;
    let mut system = system::prepare(&points, lines, constraints, parameters, feature)?;
    bindings::append(&mut system, links, parameters, feature)?;
    solve_constraints(&mut system.variables, &system.equations, feature)?;
    solved_profile(&system, plane, origin, lines, feature)
}

fn solved_profile(
    system: &system::ConstraintSystem<'_>,
    plane: SketchPlane,
    origin_mm: [f64; 3],
    lines: &[SketchLine],
    feature_id: &str,
) -> Result<SolvedSketch, ValidationError> {
    let variables = &system.variables;
    let positions = &system.positions;
    let line_map = &system.lines;
    for &(start, end) in line_map.values() {
        if (variables[start] - variables[end]).hypot(variables[start + 1] - variables[end + 1])
            <= TOLERANCE
        {
            return Err(error(ErrorCode::InvalidSketch, feature_id));
        }
    }
    let loops = profile_loops(lines, feature_id)?
        .into_iter()
        .map(|ids| {
            ids.into_iter()
                .flat_map(|id| {
                    let index = positions[id];
                    [variables[index], variables[index + 1]]
                })
                .collect()
        })
        .collect();
    let (outline_xy, holes_xy) = profile::classify(loops, feature_id)?;
    Ok(SolvedSketch {
        plane,
        origin_mm,
        outline_xy,
        holes_xy,
    })
}
