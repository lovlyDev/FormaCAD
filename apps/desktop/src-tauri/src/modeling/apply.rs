//! Host model-apply boundary. No UI runtime, no worker path supplied by IPC.
use super::{build, commit, program};
use crate::{
    agents::review_plan::ReviewPlan,
    core::{AppError, AppState, Result},
    models::Project,
};
use std::{
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

pub struct ApplyRequest {
    pub project_id: String,
    pub program: String,
    pub prompt: String,
    pub expected_revision: Option<String>,
    pub review_plan: Option<ReviewPlan>,
    pub(super) next_revision: Option<String>,
}
impl ApplyRequest {
    pub fn new(
        project_id: String,
        program: String,
        prompt: String,
        expected_revision: Option<String>,
    ) -> Self {
        Self {
            project_id,
            program,
            prompt,
            expected_revision,
            review_plan: None,
            next_revision: None,
        }
    }
}
#[derive(Debug)]
pub struct ApplyOutcome {
    pub project: Project,
    pub committed: bool,
}

pub async fn apply(state: &AppState, request: ApplyRequest) -> Result<ApplyOutcome> {
    apply_inner(state, request, None).await
}

/// Rust-only test seam. The caller cannot skip host permissions or commit checks.
#[cfg(feature = "native-occt")]
pub async fn apply_with_executable(
    state: &AppState,
    request: ApplyRequest,
    executable: &Path,
) -> Result<ApplyOutcome> {
    apply_inner(state, request, Some(executable)).await
}

fn cancelled(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Relaxed) {
        Err(AppError::Invalid("CAD_TASK_CANCELLED".into()))
    } else {
        Ok(())
    }
}
async fn current_snapshot(state: &AppState, original: &Project) -> Result<Project> {
    let latest = crate::projects::get(state, &original.id).await?;
    if latest.current_revision != original.current_revision || latest.files != original.files {
        return Err(AppError::Invalid(
            "Project changed during modeling; retry the request".into(),
        ));
    }
    Ok(latest)
}

async fn apply_inner(
    state: &AppState,
    request: ApplyRequest,
    executable: Option<&Path>,
) -> Result<ApplyOutcome> {
    let _access = crate::project_access::ensure_write(state, &request.project_id)?;
    let original = crate::projects::get(state, &request.project_id).await?;
    if request.program.trim().is_empty()
        || request.program.len() > 60000
        || request.prompt.len() > 16000
    {
        return Err(AppError::Invalid("Invalid modeling request".into()));
    }
    let prepared = program::prepare(&request.program)?;
    if original.current_revision != request.expected_revision {
        return Err(AppError::Invalid(
            "Project changed; regenerate the model from its current revision".into(),
        ));
    }
    let current = original
        .revisions
        .iter()
        .find(|revision| Some(&revision.id) == original.current_revision.as_ref());
    let review_document = request
        .review_plan
        .as_ref()
        .map(|review| {
            let candidate: crate::cad_ir::Document = serde_json::from_str(&request.program)?;
            let previous = current
                .and_then(|revision| revision.program.as_ref())
                .and_then(|source| serde_json::from_str::<crate::cad_ir::Document>(source).ok());
            review.validate(&candidate, previous.as_ref())?;
            Ok::<_, AppError>(candidate)
        })
        .transpose()?;
    if request.review_plan.is_none()
        && current.and_then(|revision| revision.program.as_deref()) == Some(request.program.trim())
    {
        return Ok(ApplyOutcome {
            project: original,
            committed: false,
        });
    }
    let job = request
        .next_revision
        .clone()
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let saved_program = commit::program_for_revision(&request.program, prepared.typed, &job)?;
    crate::permissions::consume(state, &request.project_id, "modify_project").await?;
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut tasks = state.tasks.lock().await;
        if tasks.contains_key(&request.project_id) {
            return Err(AppError::Invalid(
                "A project task is already running".into(),
            ));
        }
        tasks.insert(request.project_id.clone(), cancel.clone());
    }
    let result = async {
        let permit = state
            .cad_tasks
            .acquire(&request.project_id, "model_build", cancel.clone())
            .await?;
        // Never build an outdated captured head merely because the queue became available.
        current_snapshot(state, &original).await?;
        cancelled(&cancel)?;
        let generated = build::build(
            state,
            &original,
            &request.program,
            &prepared,
            request.review_plan.as_ref().zip(review_document.as_ref()),
            cancel.clone(),
            executable,
        )
        .await?;
        drop(permit);
        cancelled(&cancel)?;
        let _writes = state.writes.lock().await;
        cancelled(&cancel)?;
        let latest = current_snapshot(state, &original).await?;
        let project = commit::commit(
            state,
            latest,
            generated,
            job,
            saved_program,
            request.prompt.clone(),
            &cancel,
        )
        .await?;
        Ok(ApplyOutcome {
            project,
            committed: true,
        })
    }
    .await;
    // Cooperative cancellation/errors remove only this job's registration.
    // Future abort/panic cleanup is intentionally not claimed by this boundary.
    let mut tasks = state.tasks.lock().await;
    if tasks
        .get(&request.project_id)
        .is_some_and(|registered| Arc::ptr_eq(registered, &cancel))
    {
        tasks.remove(&request.project_id);
    }
    result
}
