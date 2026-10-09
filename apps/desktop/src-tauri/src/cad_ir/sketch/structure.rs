use super::*;

/// Structural checks are independent of whether this feature is currently
/// included in a body's rolled-back output. Solving is deferred to execution.
pub fn validate_structure(
    origin_mm: [f64; 3],
    points: &[SketchPoint],
    lines: &[SketchLine],
    constraints: &[SketchConstraint],
    parameters: &HashMap<&str, f64>,
    feature_id: &str,
) -> Result<(), ValidationError> {
    if points.len() < 3
        || points.len() > MAX_POINTS
        || lines.len() > MAX_LINES
        || lines.iter().filter(|line| !line.construction).count() < 3
        || constraints.len() > MAX_CONSTRAINTS
        || origin_mm
            .iter()
            .any(|value| !value.is_finite() || value.abs() > 10000.0)
    {
        return Err(error(ErrorCode::InvalidSketch, feature_id));
    }
    let mut ids = HashSet::new();
    let mut point_ids = HashSet::new();
    for point in points {
        if !valid_id(&point.id)
            || !ids.insert(point.id.as_str())
            || !point.x_mm.is_finite()
            || !point.y_mm.is_finite()
            || point.x_mm.abs() > 10000.0
            || point.y_mm.abs() > 10000.0
        {
            return Err(error(ErrorCode::InvalidSketch, feature_id));
        }
        point_ids.insert(point.id.as_str());
    }
    let mut line_ids = HashSet::new();
    let mut outgoing = HashSet::new();
    let mut incoming = HashSet::new();
    let mut used = HashSet::new();
    for line in lines {
        if !valid_id(&line.id)
            || !ids.insert(line.id.as_str())
            || !point_ids.contains(line.start_point_id.as_str())
            || !point_ids.contains(line.end_point_id.as_str())
        {
            return Err(error(ErrorCode::InvalidSketch, feature_id));
        }
        if line.start_point_id == line.end_point_id {
            return Err(error(ErrorCode::InvalidSketch, feature_id));
        }
        used.insert(line.start_point_id.as_str());
        used.insert(line.end_point_id.as_str());
        if !line.construction
            && (!outgoing.insert(line.start_point_id.as_str())
                || !incoming.insert(line.end_point_id.as_str()))
        {
            return Err(error(ErrorCode::InvalidSketch, feature_id));
        }
        line_ids.insert(line.id.as_str());
    }
    if incoming != outgoing || used.len() != points.len() {
        return Err(error(ErrorCode::InvalidSketch, feature_id));
    }
    profile_loops(lines, feature_id)?;
    for constraint in constraints {
        if !valid_id(constraint.id()) || !ids.insert(constraint.id()) {
            return Err(error(ErrorCode::InvalidSketch, feature_id));
        }
        match constraint {
            SketchConstraint::Fixed {
                point_id,
                x_mm,
                y_mm,
                ..
            } => {
                if !point_ids.contains(point_id.as_str())
                    || !x_mm.is_finite()
                    || !y_mm.is_finite()
                    || x_mm.abs() > 10000.0
                    || y_mm.abs() > 10000.0
                {
                    return Err(error(ErrorCode::InvalidSketch, feature_id));
                }
            }
            SketchConstraint::Horizontal { line_id, .. }
            | SketchConstraint::Vertical { line_id, .. } => {
                if !line_ids.contains(line_id.as_str()) {
                    return Err(error(ErrorCode::BrokenReference, feature_id));
                }
            }
            SketchConstraint::Length {
                line_id, distance, ..
            } => {
                if !line_ids.contains(line_id.as_str()) {
                    return Err(error(ErrorCode::BrokenReference, feature_id));
                }
                dimension(distance, parameters, feature_id)?;
            }
            SketchConstraint::Coincident {
                first_point_id,
                second_point_id,
                ..
            } => {
                if first_point_id == second_point_id
                    || !point_ids.contains(first_point_id.as_str())
                    || !point_ids.contains(second_point_id.as_str())
                {
                    return Err(error(ErrorCode::BrokenReference, feature_id));
                }
            }
        }
    }
    Ok(())
}
