use super::*;

pub(super) struct ConstraintSystem<'a> {
    pub variables: Vec<f64>,
    pub positions: HashMap<&'a str, usize>,
    pub lines: HashMap<&'a str, (usize, usize)>,
    pub equations: Vec<Equation>,
    pub constraint_ranges: Vec<(&'a str, std::ops::Range<usize>)>,
}

pub(super) fn prepare<'a>(
    points: &'a [SketchPoint],
    lines: &'a [SketchLine],
    constraints: &'a [SketchConstraint],
    parameters: &HashMap<&str, f64>,
    feature_id: &str,
) -> Result<ConstraintSystem<'a>, ValidationError> {
    let mut positions = HashMap::new();
    let mut variables = Vec::with_capacity(points.len() * 2);
    for point in points {
        positions.insert(point.id.as_str(), variables.len());
        variables.extend([point.x_mm, point.y_mm]);
    }
    let mut line_map = HashMap::new();
    for line in lines {
        let start = *positions
            .get(line.start_point_id.as_str())
            .ok_or_else(|| error(ErrorCode::BrokenReference, feature_id))?;
        let end = *positions
            .get(line.end_point_id.as_str())
            .ok_or_else(|| error(ErrorCode::BrokenReference, feature_id))?;
        line_map.insert(line.id.as_str(), (start, end));
    }
    let mut equations = Vec::new();
    let mut constraint_ranges = Vec::new();
    for constraint in constraints {
        let (_id, equations_for_constraint): (&str, Vec<Equation>) = match constraint {
            SketchConstraint::Fixed {
                id,
                point_id,
                x_mm,
                y_mm,
            } => {
                let point = *positions
                    .get(point_id.as_str())
                    .ok_or_else(|| error(ErrorCode::BrokenReference, feature_id))?;
                if !x_mm.is_finite()
                    || !y_mm.is_finite()
                    || x_mm.abs() > 10000.0
                    || y_mm.abs() > 10000.0
                {
                    return Err(error(ErrorCode::InvalidValue, feature_id));
                }
                (
                    id,
                    vec![
                        Equation::Fixed(point, *x_mm),
                        Equation::Fixed(point + 1, *y_mm),
                    ],
                )
            }
            SketchConstraint::Horizontal { id, line_id }
            | SketchConstraint::Vertical { id, line_id } => {
                let &(start, end) = line_map
                    .get(line_id.as_str())
                    .ok_or_else(|| error(ErrorCode::BrokenReference, feature_id))?;
                let axis = usize::from(matches!(constraint, SketchConstraint::Horizontal { .. }));
                (id, vec![Equation::Equal(start + axis, end + axis)])
            }
            SketchConstraint::Length {
                id,
                line_id,
                distance,
            } => {
                let &(start, end) = line_map
                    .get(line_id.as_str())
                    .ok_or_else(|| error(ErrorCode::BrokenReference, feature_id))?;
                (
                    id,
                    vec![Equation::Distance(
                        start,
                        end,
                        dimension(distance, parameters, feature_id)?,
                    )],
                )
            }
            SketchConstraint::Coincident {
                id,
                first_point_id,
                second_point_id,
            } => {
                let first = *positions
                    .get(first_point_id.as_str())
                    .ok_or_else(|| error(ErrorCode::BrokenReference, feature_id))?;
                let second = *positions
                    .get(second_point_id.as_str())
                    .ok_or_else(|| error(ErrorCode::BrokenReference, feature_id))?;
                if first == second {
                    return Err(error(ErrorCode::InvalidSketch, feature_id));
                }
                (
                    id,
                    vec![
                        Equation::Equal(first, second),
                        Equation::Equal(first + 1, second + 1),
                    ],
                )
            }
        };
        let start = equations.len();
        equations.extend(equations_for_constraint);
        constraint_ranges.push((_id, start..equations.len()));
    }
    Ok(ConstraintSystem {
        variables,
        positions,
        lines: line_map,
        equations,
        constraint_ranges,
    })
}
