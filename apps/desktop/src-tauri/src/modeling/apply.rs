//! Host model-apply boundary. No UI runtime, no worker path supplied by IPC.
use super::repository_apply::{self, managed::ManagedHost, ApplyRuntime, RepositoryApply};
use crate::{
    agents::review_plan::ReviewPlan,
    core::{AppState, Result},
    models::Project,
};
use std::path::Path;
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
    run_managed(state, request, None).await
}

/// Rust-only test seam. The caller cannot skip host permissions or commit checks.
#[cfg(feature = "native-worker-client")]
pub async fn apply_with_executable(
    state: &AppState,
    request: ApplyRequest,
    executable: &Path,
) -> Result<ApplyOutcome> {
    run_managed(state, request, Some(executable)).await
}

async fn run_managed(
    state: &AppState,
    request: ApplyRequest,
    executable: Option<&Path>,
) -> Result<ApplyOutcome> {
    let project_id = request.project_id.clone();
    let host = ManagedHost {
        state,
        project_id: &project_id,
        executable,
    };
    match repository_apply::apply(ApplyRuntime::from_state(state), &host, request).await? {
        RepositoryApply::NoOp(project) => Ok(ApplyOutcome {
            project,
            committed: false,
        }),
        RepositoryApply::Published(project) => Ok(ApplyOutcome {
            project,
            committed: true,
        }),
    }
}
