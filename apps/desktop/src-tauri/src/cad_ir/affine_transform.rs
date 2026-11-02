use super::{ErrorCode, Result, ValidationError};

/// Same row-major 3x4 contract as the native BREP primitive, not a renderer matrix.
pub(super) fn validate(matrix: &[f64; 12], target: &str) -> Result<()> {
    let determinant = matrix[0] * (matrix[5] * matrix[10] - matrix[6] * matrix[9])
        - matrix[1] * (matrix[4] * matrix[10] - matrix[6] * matrix[8])
        + matrix[2] * (matrix[4] * matrix[9] - matrix[5] * matrix[8]);
    if matrix.iter().enumerate().any(|(i, value)| {
        !value.is_finite() || value.abs() > if i % 4 == 3 { 10000.0 } else { 1000.0 }
    }) || !determinant.is_finite()
        || determinant <= 1e-18
    {
        return Err(ValidationError::new(
            ErrorCode::InvalidValue,
            Some(target.to_owned()),
        ));
    }
    Ok(())
}

pub(super) fn is_identity(matrix: &[f64; 12]) -> bool {
    let identity = [1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0.];
    matrix
        .iter()
        .zip(identity)
        .all(|(a, b)| (*a - b).abs() < 1e-12)
}
