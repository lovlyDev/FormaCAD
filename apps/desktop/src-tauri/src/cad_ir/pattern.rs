use super::{transforms::validate_axis, ErrorCode, Result, ValidationError, MAX_DIMENSION_MM};
use serde::{Deserialize, Serialize};

const MAX_INSTANCES: u32 = 128;
const MIN_STEP: f64 = 1e-7;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LinearPattern {
    pub count_u: u32,
    pub step_u_mm: [f64; 3],
    pub count_v: u32,
    pub step_v_mm: [f64; 3],
}

impl LinearPattern {
    pub(super) fn validate(&self, id: &str) -> Result<()> {
        let invalid = || ValidationError::new(ErrorCode::InvalidValue, Some(id.to_owned()));
        if !(2..=MAX_INSTANCES).contains(&self.count_u)
            || !(1..=MAX_INSTANCES / self.count_u).contains(&self.count_v)
        {
            return Err(invalid());
        }
        let length = |step: [f64; 3]| step.iter().map(|value| value * value).sum::<f64>().sqrt();
        let u = length(self.step_u_mm);
        let v = length(self.step_v_mm);
        if !u.is_finite()
            || !v.is_finite()
            || u <= MIN_STEP
            || u * f64::from(self.count_u - 1) > MAX_DIMENSION_MM
            || v > MAX_DIMENSION_MM
        {
            return Err(invalid());
        }
        if self.count_v > 1 {
            let [ux, uy, uz] = self.step_u_mm;
            let [vx, vy, vz] = self.step_v_mm;
            let cross = length([uy * vz - uz * vy, uz * vx - ux * vz, ux * vy - uy * vx]);
            if v <= MIN_STEP
                || v * f64::from(self.count_v - 1) > MAX_DIMENSION_MM
                || cross <= MIN_STEP * u * v
            {
                return Err(invalid());
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CircularPattern {
    pub count: u32,
    pub axis_origin_mm: [f64; 3],
    pub axis_direction: [f64; 3],
    pub step_deg: f64,
}

impl CircularPattern {
    pub(super) fn validate(&self, id: &str) -> Result<()> {
        validate_axis(self.axis_origin_mm, self.axis_direction, id)?;
        if !(2..=MAX_INSTANCES).contains(&self.count)
            || !self.step_deg.is_finite()
            || self.step_deg.abs() <= MIN_STEP
            || self.step_deg.abs() * f64::from(self.count - 1) >= 360. - MIN_STEP
        {
            return Err(ValidationError::new(
                ErrorCode::InvalidValue,
                Some(id.to_owned()),
            ));
        }
        Ok(())
    }
}
