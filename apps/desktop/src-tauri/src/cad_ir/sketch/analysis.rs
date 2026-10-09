//! Read-only diagnostics. The rank is local to the current solved configuration.
use super::*;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConstraintResidual {
    pub id: String,
    pub residual_mm: f64,
    pub satisfied: bool,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SketchAnalysis {
    pub status: &'static str,
    pub error_code: Option<ErrorCode>,
    pub degrees_of_freedom: Option<usize>,
    pub redundant_equations: Option<usize>,
    pub max_residual_mm: Option<f64>,
    pub constraints: Vec<ConstraintResidual>,
    pub solved_points: Vec<SketchPoint>,
    pub loop_count: usize,
    pub hole_count: usize,
    pub profile_area_mm2: Option<f64>,
}

fn rank(mut rows: Vec<Vec<f64>>, columns: usize) -> usize {
    // Normalize rows so dimensions and coordinate scale do not bias the pivot tolerance.
    for row in &mut rows {
        let norm = row.iter().map(|v| v * v).sum::<f64>().sqrt();
        if norm > 1e-12 {
            for value in row {
                *value /= norm;
            }
        }
    }
    let mut pivot_row = 0;
    for col in 0..columns {
        if pivot_row == rows.len() {
            break;
        }
        let best = (pivot_row..rows.len())
            .max_by(|&a, &b| rows[a][col].abs().total_cmp(&rows[b][col].abs()))
            .unwrap();
        if rows[best][col].abs() < 1e-8 {
            continue;
        }
        rows.swap(pivot_row, best);
        let pivot = rows[pivot_row][col];
        for value in &mut rows[pivot_row][col..] {
            *value /= pivot;
        }
        let (processed, remaining) = rows.split_at_mut(pivot_row + 1);
        let normalized = &processed[pivot_row];
        for row in remaining {
            let factor = row[col];
            for (value, pivot_value) in row[col..].iter_mut().zip(&normalized[col..]) {
                *value -= factor * pivot_value;
            }
        }
        pivot_row += 1;
    }
    pivot_row
}

pub fn analyze(
    plane: SketchPlane,
    origin_mm: [f64; 3],
    points: &[SketchPoint],
    lines: &[SketchLine],
    constraints: &[SketchConstraint],
    parameters: &HashMap<&str, f64>,
    feature_id: &str,
) -> SketchAnalysis {
    analyze_bound(
        plane,
        origin_mm,
        points,
        lines,
        constraints,
        &[],
        parameters,
        feature_id,
    )
}

// Explicit sketch geometry and parameter context are passed together at the IR boundary.
#[allow(clippy::too_many_arguments)]
pub fn analyze_bound(
    plane: SketchPlane,
    origin_mm: [f64; 3],
    points: &[SketchPoint],
    lines: &[SketchLine],
    constraints: &[SketchConstraint],
    links: &[bindings::CoordinateBinding],
    parameters: &HashMap<&str, f64>,
    feature_id: &str,
) -> SketchAnalysis {
    let mut report = SketchAnalysis {
        status: "invalidStructure",
        error_code: None,
        degrees_of_freedom: None,
        redundant_equations: None,
        max_residual_mm: None,
        constraints: Vec::new(),
        solved_points: Vec::new(),
        loop_count: 0,
        hole_count: 0,
        profile_area_mm2: None,
    };
    let (origin_mm, resolved) = match bindings::resolve(
        origin_mm,
        points,
        constraints,
        links,
        parameters,
        feature_id,
    ) {
        Ok(value) => value,
        Err(error) => {
            report.error_code = Some(error.code);
            return report;
        }
    };
    let points = resolved.as_slice();
    if let Err(error) = validate_structure(
        origin_mm,
        points,
        lines,
        constraints,
        parameters,
        feature_id,
    ) {
        report.error_code = Some(error.code);
        return report;
    }
    let mut system = match system::prepare(points, lines, constraints, parameters, feature_id) {
        Ok(system) => system,
        Err(error) => {
            report.error_code = Some(error.code);
            return report;
        }
    };
    if let Err(error) = bindings::append(&mut system, links, parameters, feature_id) {
        report.error_code = Some(error.code);
        return report;
    }
    let result = solve_constraints(&mut system.variables, &system.equations, feature_id);
    report.constraints = system
        .constraint_ranges
        .iter()
        .map(|(id, range)| {
            let residual_mm = range
                .clone()
                .map(|i| system.equations[i].evaluate(&system.variables).0.abs())
                .fold(0.0, f64::max);
            ConstraintResidual {
                id: (*id).into(),
                residual_mm,
                satisfied: residual_mm <= TOLERANCE,
            }
        })
        .collect();
    report.max_residual_mm = Some(
        report
            .constraints
            .iter()
            .map(|item| item.residual_mm)
            .fold(0.0, f64::max),
    );
    if let Err(error) = result {
        report.status = "conflict";
        report.error_code = Some(error.code);
        return report;
    }
    let solved = match solved_profile(&system, plane, origin_mm, lines, feature_id) {
        Ok(solved) => solved,
        Err(error) => {
            report.status = "invalidProfile";
            report.error_code = Some(error.code);
            return report;
        }
    };
    let rows: Vec<_> = system
        .equations
        .iter()
        .map(|equation| {
            let mut row = vec![0.0; system.variables.len()];
            for (i, derivative) in equation.evaluate(&system.variables).1 {
                row[i] += derivative;
            }
            row
        })
        .collect();
    let rank = rank(rows, system.variables.len());
    report.degrees_of_freedom = Some(system.variables.len() - rank);
    report.redundant_equations = Some(system.equations.len() - rank);
    report.solved_points = points
        .iter()
        .map(|point| {
            let index = system.positions[point.id.as_str()];
            SketchPoint {
                id: point.id.clone(),
                x_mm: system.variables[index],
                y_mm: system.variables[index + 1],
            }
        })
        .collect();
    let area = |outline: &[f64]| {
        let vertices: Vec<_> = outline.chunks_exact(2).collect();
        (0..vertices.len())
            .map(|i| {
                let a = vertices[i];
                let b = vertices[(i + 1) % vertices.len()];
                a[0] * b[1] - b[0] * a[1]
            })
            .sum::<f64>()
            .abs()
            / 2.0
    };
    report.profile_area_mm2 =
        Some(area(&solved.outline_xy) - solved.holes_xy.iter().map(|hole| area(hole)).sum::<f64>());
    report.hole_count = solved.holes_xy.len();
    report.loop_count = report.hole_count + 1;
    report.status = "solved";
    report
}

#[cfg(test)]
mod tests;
