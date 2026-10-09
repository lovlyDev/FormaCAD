//! At most two geometry corrections against one captured committed project.
mod engine;
#[cfg(test)]
mod tests;
use super::{protocol::parse_native_output, PlanResult};
use crate::{
    core::{AppError, AppState, Result},
    models::Project,
    processes::{self, CommandSpec, OutputListener},
};
use std::{
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

fn geometry_feedback(error: &AppError) -> Option<String> {
    let AppError::Invalid(raw) = error else {
        return None;
    };
    let value: serde_json::Value = serde_json::from_str(raw).ok()?;
    let code = value.get("code")?.as_str()?;
    if !matches!(
        code,
        "FILLET_FAILED"
            | "FILLET_NO_EDGES"
            | "CHAMFER_FAILED"
            | "CHAMFER_NO_EDGES"
            | "BOOLEAN_FAILED"
            | "INVALID_DIMENSION"
            | "INVALID_SKETCH"
            | "INVALID_TRANSFORM"
            | "SKETCH_CONSTRAINT_CONFLICT"
            | "INVALID_BODY"
            | "MISSING_BODY"
            | "BROKEN_REFERENCE"
            | "REFERENCE_TYPE_MISMATCH"
            | "UNSUPPORTED_SUPPRESSION"
            | "CAD_IR_INVALID"
            | "OCCT_ERROR"
            | "NATIVE_ERROR"
            | "EXPECTED_CHECK_FAILED"
    ) {
        return None;
    }
    let message = value
        .get("message")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let mut feedback =
        serde_json::json!({"code":code,"detail":message.chars().take(300).collect::<String>()});
    if code == "EXPECTED_CHECK_FAILED" {
        if let Some(detail) = value.get("detail") {
            let text = detail.to_string();
            feedback["check"] = text.chars().take(1000).collect::<String>().into();
        }
    }
    Some(feedback.to_string())
}

pub(super) struct RepairSnapshot<'a> {
    pub current: Option<&'a crate::cad_ir::Document>,
    pub state: &'a AppState,
    pub captured: &'a Project,
}

pub async fn plan_with_two_repairs(
    provider: &str,
    spec: &CommandSpec,
    initial_input: &str,
    cancel: Arc<AtomicBool>,
    listener: OutputListener,
    snapshot: RepairSnapshot<'_>,
) -> Result<PlanResult> {
    let RepairSnapshot {
        current,
        state,
        captured,
    } = snapshot;
    let plan_cancel = cancel.clone();
    let build_cancel = cancel.clone();
    engine::run(
        initial_input,
        cancel,
        || async {
            let _access = crate::project_access::ensure_write(state, &captured.id)?;
            let actual = crate::projects::get(state, &captured.id).await?;
            if actual.current_revision != captured.current_revision {
                return Err(AppError::Invalid(
                    "Project changed; retry the CAD edit".into(),
                ));
            }
            Ok(())
        },
        |input| {
            let cancel = plan_cancel.clone();
            let listener = listener.clone();
            async move {
                let output = processes::run_stream(
                    spec,
                    &input,
                    cancel,
                    Duration::from_secs(180),
                    Some(listener),
                )
                .await?;
                match current {
                    Some(current) => {
                        super::protocol::parse_native_edit_output(provider, &output, current)
                    }
                    None => parse_native_output(provider, &output),
                }
            }
        },
        |plan, attempt| {
            let cancel = build_cancel.clone();
            async move {
                let candidate_dir = spec.cwd.join(format!("candidate-{attempt}"));
                std::fs::create_dir(&candidate_dir)?;
                let built = async {
                    let program = plan.program.as_deref().ok_or_else(|| {
                        AppError::Invalid("Native agent response must contain CAD IR v2".into())
                    })?;
                    let document: crate::cad_ir::Document = serde_json::from_str(program)?;
                    if let Some(review) = &plan.review_plan {
                        review.validate(&document, current)?;
                    }
                    crate::native::assets::stage_project_assets(
                        state,
                        captured,
                        &document,
                        &candidate_dir,
                    )?;
                    let _cad = state
                        .cad_tasks
                        .acquire(&captured.id, "ai_validation", cancel.clone())
                        .await?;
                    let built =
                        crate::native::worker::build_if_ir_v2(program, &candidate_dir, cancel)
                            .await?;
                    if built {
                        if let Some(review) = &plan.review_plan {
                            let result_path = crate::security::guarded(
                                &candidate_dir,
                                std::path::Path::new("result.json"),
                            )?;
                            if std::fs::metadata(&result_path)?.len() > 65536 {
                                return Err(AppError::Invalid(
                                    "CAD worker response is too large".into(),
                                ));
                            }
                            let result = serde_json::from_slice(&std::fs::read(result_path)?)?;
                            super::review_plan::check_metrics(review, &document, &result)?;
                        }
                    }
                    Ok(built)
                }
                .await;
                let _ = std::fs::remove_dir_all(&candidate_dir);
                built
            }
        },
    )
    .await
}
