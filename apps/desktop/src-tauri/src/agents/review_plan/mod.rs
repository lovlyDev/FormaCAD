//! Bounded provider claims and independently evaluated candidate expectations.
mod metrics;
#[cfg(test)]
mod tests;
use crate::{
    cad_ir::Document,
    core::{AppError, Result},
};
pub use metrics::check_metrics;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReviewPlan {
    pub assumptions: Vec<String>,
    pub dimensions: Vec<PlannedDimension>,
    pub affected_body_ids: Vec<String>,
    pub expected_checks: Vec<ExpectedCheck>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlannedDimension {
    pub name: String,
    pub value_mm: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parameter_id: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ExpectedCheck {
    ValidSolid,
    BodyCount {
        count: usize,
    },
    Bounds {
        size_mm: [f64; 3],
        tolerance_mm: f64,
    },
    Volume {
        value_mm3: f64,
        tolerance_mm3: f64,
    },
}
fn invalid() -> AppError {
    AppError::Invalid(
        serde_json::json!({"code":"REVIEW_PLAN_INVALID","message":"AI review plan is invalid."})
            .to_string(),
    )
}
fn text(value: &str, max: usize) -> bool {
    !value.trim().is_empty() && value.chars().count() <= max
}
fn positive(value: f64) -> bool {
    value.is_finite() && value > 0.0
}

impl ReviewPlan {
    pub fn validate(&self, candidate: &Document, current: Option<&Document>) -> Result<()> {
        if self.assumptions.len() > 16
            || self.dimensions.len() > 32
            || self.affected_body_ids.len() > 32
            || self.expected_checks.len() > 16
            || self.assumptions.iter().any(|value| !text(value, 400))
        {
            return Err(invalid());
        }
        let mut dimensions = HashSet::new();
        for dimension in &self.dimensions {
            if !text(&dimension.name, 80)
                || !dimensions.insert(&dimension.name)
                || !dimension.value_mm.is_finite()
                || !(0.0..=10000.0).contains(&dimension.value_mm)
            {
                return Err(invalid());
            }
            if let Some(id) = &dimension.parameter_id {
                let parameter = candidate
                    .parameters
                    .iter()
                    .find(|parameter| &parameter.id == id)
                    .ok_or_else(invalid)?;
                if parameter.value_mm != dimension.value_mm {
                    return Err(invalid());
                }
            }
        }
        let mut bodies = HashSet::new();
        for id in &self.affected_body_ids {
            if !bodies.insert(id)
                || !candidate.bodies.iter().any(|body| &body.id == id)
                    && !current
                        .is_some_and(|current| current.bodies.iter().any(|body| &body.id == id))
            {
                return Err(invalid());
            }
        }
        for check in &self.expected_checks {
            let valid = match check {
                ExpectedCheck::ValidSolid => true,
                ExpectedCheck::BodyCount { count } => (1..=32).contains(count),
                ExpectedCheck::Bounds {
                    size_mm,
                    tolerance_mm,
                } => {
                    size_mm
                        .iter()
                        .all(|value| positive(*value) && *value <= 20000.0)
                        && tolerance_mm.is_finite()
                        && (0.0..=10.0).contains(tolerance_mm)
                }
                ExpectedCheck::Volume {
                    value_mm3,
                    tolerance_mm3,
                } => {
                    positive(*value_mm3)
                        && *value_mm3 <= 8.0e12
                        && tolerance_mm3.is_finite()
                        && *tolerance_mm3 >= 0.0
                        && *tolerance_mm3 <= *value_mm3
                }
            };
            if !valid {
                return Err(invalid());
            }
        }
        Ok(())
    }
}

pub(super) fn parse(
    value: Option<&serde_json::Value>,
    candidate: Option<&Document>,
    current: Option<&Document>,
) -> Result<Option<ReviewPlan>> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    let candidate = candidate.ok_or_else(invalid)?;
    let plan: ReviewPlan = serde_json::from_value(value.clone()).map_err(|_| invalid())?;
    plan.validate(candidate, current)?;
    Ok(Some(plan))
}
