//! A fixed-budget candidate state machine: no project persistence or grants.
use super::geometry_feedback;
use crate::{
    agents::PlanResult,
    core::{AppError, Result},
};
use std::{
    future::Future,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
const MAX_CORRECTIONS: usize = 2;
fn not_cancelled(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Relaxed) {
        return Err(AppError::Invalid(
            "Task cancelled. Your last saved revision is unchanged.".into(),
        ));
    }
    Ok(())
}
pub(super) async fn run<G, GF, P, PF, B, BF>(
    initial_input: &str,
    cancel: Arc<AtomicBool>,
    mut guard: G,
    mut plan: P,
    mut build: B,
) -> Result<PlanResult>
where
    G: FnMut() -> GF,
    GF: Future<Output = Result<()>>,
    P: FnMut(String) -> PF,
    PF: Future<Output = Result<PlanResult>>,
    B: FnMut(PlanResult, usize) -> BF,
    BF: Future<Output = Result<bool>>,
{
    let mut input = initial_input.to_owned();
    let mut original_plan = None;
    for attempt in 0..=MAX_CORRECTIONS {
        not_cancelled(&cancel)?;
        guard().await?;
        let mut candidate = plan(input).await?;
        let expected = original_plan.get_or_insert_with(|| candidate.review_plan.clone());
        candidate.review_plan = expected.clone();
        not_cancelled(&cancel)?;
        guard().await?;
        if candidate.program.is_none() {
            return Ok(candidate);
        }
        let built = build(candidate.clone(), attempt).await;
        not_cancelled(&cancel)?;
        guard().await?;
        match built {
            Ok(true) => return Ok(candidate),
            Ok(false) => {
                return Err(AppError::Invalid(
                    "Native agent response must contain CAD IR v2".into(),
                ))
            }
            Err(error) => {
                let Some(feedback) = geometry_feedback(&error) else {
                    return Err(error);
                };
                if attempt == MAX_CORRECTIONS {
                    return Err(AppError::Invalid(
                        serde_json::json!({
                            "code":"CAD_REPAIR_EXHAUSTED",
                            "message":"Model could not be built after two correction attempts.",
                            "detail":feedback
                        })
                        .to_string(),
                    ));
                }
                let frozen_plan = serde_json::to_string(&candidate.review_plan)?;
                input = format!("{initial_input}\n\nHOST CAD VALIDATION FAILED. Correction {} of {MAX_CORRECTIONS}. No candidate was saved; use the same original committed model and stable IDs. Initial reviewPlan (immutable across corrections): {frozen_plan}. Latest error: {feedback}. Return a corrected complete CAD IR v2 document or a command batch against the ORIGINAL expectedRevision. Do not repeat the invalid operation unchanged.", attempt + 1);
            }
        }
    }
    unreachable!("bounded candidate loop always returns")
}
