//! Snapshot-based modeling intent preparation, never permission or commit authority.
//! Keep request validation, ReviewPlan/no-op decisions and saved revision binding
//! shared by managed and future folder hosts. UUIDs are sampled only for real work.
use super::{
    apply::ApplyRequest,
    commit,
    program::{self, PreparedProgram},
};
use crate::{
    cad_ir::Document,
    core::{AppError, Result},
    models::Project,
};

pub(crate) enum PreparedRequest {
    NoOp,
    Apply(Box<PreparedApply>),
}

/// Owned validated intent, not a captured source, grant, task or saved result.
/// ReviewPlan's candidate keeps its requested revision; saved_program alone is
/// rebound through the existing commit::program_for_revision implementation.
pub(crate) struct PreparedApply {
    program: PreparedProgram,
    review_document: Option<Document>,
    job: String,
    saved_program: String,
}

impl PreparedApply {
    pub(crate) fn program(&self) -> &PreparedProgram {
        &self.program
    }
    pub(crate) fn review_document(&self) -> Option<&Document> {
        self.review_document.as_ref()
    }
    pub(super) fn into_commit_parts(self) -> (String, String) {
        (self.job, self.saved_program)
    }
}

pub(crate) fn prepare(original: &Project, request: &ApplyRequest) -> Result<PreparedRequest> {
    prepare_with_job(original, request, || uuid::Uuid::new_v4().to_string())
}

fn prepare_with_job(
    original: &Project,
    request: &ApplyRequest,
    next_job: impl FnOnce() -> String,
) -> Result<PreparedRequest> {
    if request.program.trim().is_empty()
        || request.program.len() > 60000
        || request.prompt.len() > 16000
    {
        return Err(AppError::Invalid("Invalid modeling request".into()));
    }
    let program = program::prepare(&request.program)?;
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
            let candidate: Document = serde_json::from_str(&request.program)?;
            let previous = current
                .and_then(|revision| revision.program.as_ref())
                .and_then(|source| serde_json::from_str::<Document>(source).ok());
            review.validate(&candidate, previous.as_ref())?;
            Ok::<_, AppError>(candidate)
        })
        .transpose()?;
    if request.review_plan.is_none()
        && current.and_then(|revision| revision.program.as_deref()) == Some(request.program.trim())
    {
        return Ok(PreparedRequest::NoOp);
    }
    let job = request.next_revision.clone().unwrap_or_else(next_job);
    let saved_program = commit::program_for_revision(&request.program, program.typed, &job)?;
    Ok(PreparedRequest::Apply(Box::new(PreparedApply {
        program,
        review_document,
        job,
        saved_program,
    })))
}

#[cfg(test)]
mod tests;
