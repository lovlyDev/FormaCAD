//! Unit-aware bounds for rigid transformations. Angles are degrees, not lengths.
use super::{ErrorCode, Result, ValidationError, MAX_DIMENSION_MM};

pub(super) fn validate_axis(origin: [f64; 3], direction: [f64; 3], id: &str) -> Result<()> {
    if origin
        .iter()
        .any(|v| !v.is_finite() || v.abs() > MAX_DIMENSION_MM)
        || direction.iter().any(|v| !v.is_finite() || v.abs() > 1e6)
        || direction.iter().map(|v| v * v).sum::<f64>() < 1e-18
    {
        return Err(ValidationError::new(
            ErrorCode::InvalidValue,
            Some(id.to_owned()),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn axis_bounds_match_kernel_and_units_do_not_mix() {
        for origin in [[10_001.0, 0.0, 0.0], [f64::INFINITY, 0.0, 0.0]] {
            assert_eq!(
                validate_axis(origin, [1.0, 0.0, 0.0], "rotation")
                    .unwrap_err()
                    .code,
                ErrorCode::InvalidValue
            );
        }
        for direction in [
            [0.0; 3],
            [1e-10, 0.0, 0.0],
            [1e7, 0.0, 0.0],
            [0.0, f64::NAN, 1.0],
        ] {
            let error = validate_axis([0.0; 3], direction, "mirror").unwrap_err();
            assert_eq!(error.code, ErrorCode::InvalidValue);
            assert_eq!(error.target_id.as_deref(), Some("mirror"));
        }
        validate_axis([10_000.0, -10_000.0, 0.0], [1e-9, 0.0, 0.0], "rotation").unwrap();
        validate_axis([0.0; 3], [1e6, -1e6, 1e6], "mirror").unwrap();
    }
}
