//! Resolve a bounded AI command proposal against the captured revision, without persistence.
use super::PlanResult;
use crate::{
    cad_ir::{self, Command, Document, ErrorCode, ValidationError},
    core::{AppError, Result},
};
use serde::Deserialize;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Proposal {
    message: String,
    expected_revision: String,
    commands: Vec<Command>,
    #[serde(default)]
    review_plan: Option<super::super::review_plan::ReviewPlan>,
}
pub(super) fn stage(value: &serde_json::Value, current: Option<&Document>) -> Result<PlanResult> {
    let invalid = || AppError::Invalid("Agent response was not valid model data".into());
    let list = value
        .get("commands")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(invalid)?;
    if list.len() > 256 {
        return Err(cad_ir::app_error(ValidationError {
            code: ErrorCode::InvalidDocument,
            target_id: None,
        }));
    }
    let proposal: Proposal = serde_json::from_value(value.clone()).map_err(|_| invalid())?;
    let current = current.ok_or_else(|| {
        cad_ir::app_error(ValidationError {
            code: ErrorCode::MissingTarget,
            target_id: None,
        })
    })?;
    let next = uuid::Uuid::new_v4().to_string();
    let document = cad_ir::apply_commands(
        current,
        &proposal.expected_revision,
        &next,
        &proposal.commands,
    )
    .map_err(cad_ir::app_error)?;
    let program = serde_json::to_string_pretty(&document)?;
    if program.len() > 60000 {
        return Err(AppError::Invalid("CAD document exceeds 60000 bytes".into()));
    }
    if let Some(plan) = &proposal.review_plan {
        plan.validate(&document, Some(current))?;
    }
    Ok(PlanResult {
        message: proposal.message,
        program: Some(program),
        review_plan: proposal.review_plan,
    })
}
