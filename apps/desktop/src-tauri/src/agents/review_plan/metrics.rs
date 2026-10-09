use super::{ExpectedCheck, ReviewPlan};
use crate::{
    cad_ir::Document,
    core::{AppError, Result},
};

fn failed(check: &ExpectedCheck, document: &Document, result: &serde_json::Value) -> AppError {
    AppError::Invalid(
        serde_json::json!({
            "code":"EXPECTED_CHECK_FAILED",
            "message":"AI candidate does not satisfy the expected checks.",
        "detail":{"expected":check,"actual":{"volumeMm3":result["volumeMm3"],"boundsMm":result["boundsMm"],"bodyCount":document.bodies.len()}}
        })
        .to_string(),
    )
}
/// Call only on the fresh, verified worker response for this exact candidate;
/// displayed preview metrics and provider claims are not commit evidence.
pub fn check_metrics(
    plan: &ReviewPlan,
    document: &Document,
    result: &serde_json::Value,
) -> Result<()> {
    let volume = result["volumeMm3"]
        .as_f64()
        .filter(|v| v.is_finite() && *v > 0.0);
    let bounds = result["boundsMm"].as_array().and_then(|values| {
        if values.len() != 3 {
            return None;
        }
        let mut bounds = [0.0; 3];
        for (index, value) in values.iter().enumerate() {
            bounds[index] = value.as_f64().filter(|v| v.is_finite() && *v > 0.0)?;
        }
        Some(bounds)
    });
    for check in &plan.expected_checks {
        let success = result["status"] == "completed"
            && match check {
                ExpectedCheck::ValidSolid => volume.is_some() && bounds.is_some(),
                ExpectedCheck::BodyCount { count } => document.bodies.len() == *count,
                ExpectedCheck::Bounds {
                    size_mm,
                    tolerance_mm,
                } => bounds.is_some_and(|bounds| {
                    bounds
                        .iter()
                        .zip(size_mm)
                        .all(|(actual, expected)| (actual - expected).abs() <= *tolerance_mm)
                }),
                ExpectedCheck::Volume {
                    value_mm3,
                    tolerance_mm3,
                } => volume.is_some_and(|volume| (volume - value_mm3).abs() <= *tolerance_mm3),
            };
        if !success {
            return Err(failed(check, document, result));
        }
    }
    Ok(())
}
