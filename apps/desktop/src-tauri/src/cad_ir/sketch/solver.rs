use super::*;

#[derive(Clone)]
pub(super) enum Equation {
    Fixed(usize, f64),
    Equal(usize, usize),
    Distance(usize, usize, f64),
}

impl Equation {
    pub(super) fn evaluate(&self, variables: &[f64]) -> (f64, Vec<(usize, f64)>) {
        match *self {
            Self::Fixed(index, value) => (variables[index] - value, vec![(index, 1.0)]),
            Self::Equal(first, second) => (
                variables[first] - variables[second],
                vec![(first, 1.0), (second, -1.0)],
            ),
            Self::Distance(first, second, value) => {
                let dx = variables[first] - variables[second];
                let dy = variables[first + 1] - variables[second + 1];
                let length = dx.hypot(dy);
                let scale = if length < 1e-12 { 0.0 } else { 1.0 / length };
                (
                    length - value,
                    vec![
                        (first, dx * scale),
                        (first + 1, dy * scale),
                        (second, -dx * scale),
                        (second + 1, -dy * scale),
                    ],
                )
            }
        }
    }
}

fn residual(variables: &[f64], equations: &[Equation]) -> f64 {
    equations
        .iter()
        .map(|equation| equation.evaluate(variables).0.abs())
        .fold(0.0, f64::max)
}

fn solve_linear(mut matrix: Vec<Vec<f64>>, mut rhs: Vec<f64>) -> Option<Vec<f64>> {
    let count = rhs.len();
    for column in 0..count {
        let pivot = (column..count)
            .max_by(|&a, &b| matrix[a][column].abs().total_cmp(&matrix[b][column].abs()))?;
        if matrix[pivot][column].abs() < 1e-15 {
            return None;
        }
        matrix.swap(column, pivot);
        rhs.swap(column, pivot);
        let (pivot_rows, remaining_rows) = matrix.split_at_mut(column + 1);
        let pivot_row = &pivot_rows[column];
        let pivot_rhs = rhs[column];
        for (offset, row) in remaining_rows.iter_mut().enumerate() {
            let factor = row[column] / pivot_row[column];
            for (cell, pivot_cell) in row.iter_mut().zip(pivot_row).skip(column + 1) {
                *cell -= factor * pivot_cell;
            }
            rhs[column + 1 + offset] -= factor * pivot_rhs;
        }
    }
    let mut solution = vec![0.0; count];
    for row in (0..count).rev() {
        solution[row] = (rhs[row]
            - (row + 1..count)
                .map(|cell| matrix[row][cell] * solution[cell])
                .sum::<f64>())
            / matrix[row][row];
    }
    Some(solution)
}

pub(super) fn solve_constraints(
    variables: &mut [f64],
    equations: &[Equation],
    feature_id: &str,
) -> Result<(), ValidationError> {
    let count = variables.len();
    let mut damping = 1e-5;
    for _ in 0..60 {
        let current = residual(variables, equations);
        if current <= TOLERANCE {
            return Ok(());
        }
        let mut matrix = vec![vec![0.0; count]; count];
        let mut gradient = vec![0.0; count];
        for equation in equations {
            let (value, jacobian) = equation.evaluate(variables);
            for &(row, derivative) in &jacobian {
                gradient[row] -= derivative * value;
                for &(column, other) in &jacobian {
                    matrix[row][column] += derivative * other;
                }
            }
        }
        for (index, row) in matrix.iter_mut().enumerate() {
            row[index] += damping;
        }
        let Some(delta) = solve_linear(matrix, gradient) else {
            break;
        };
        let trial: Vec<_> = variables
            .iter()
            .zip(delta)
            .map(|(value, step)| value + step)
            .collect();
        if trial
            .iter()
            .all(|value| value.is_finite() && value.abs() <= 10000.0)
            && residual(&trial, equations) < current
        {
            variables.copy_from_slice(&trial);
            damping = (damping * 0.3).max(1e-9);
        } else {
            damping *= 10.0;
        }
    }
    Err(error(ErrorCode::SketchConstraintConflict, feature_id))
}
