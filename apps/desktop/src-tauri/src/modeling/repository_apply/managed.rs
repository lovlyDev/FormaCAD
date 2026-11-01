//! Managed source adapter; never used to authorize or store external folders.
use super::{ApplyHost, GeneratedModel, PreparedApply, PreparedModelChange};
use crate::{
    core::{AppError, AppState, Result},
    modeling::{apply::ApplyRequest, build, commit},
    models::Project,
    project_access::WriteGuard,
    project_repository::legacy::{self, CommittedSnapshot},
};
use std::{
    path::Path,
    sync::{atomic::AtomicBool, Arc},
};
use tokio::sync::MutexGuard;

pub(in crate::modeling) struct ManagedHost<'a> {
    pub state: &'a AppState,
    pub project_id: &'a str,
    pub executable: Option<&'a Path>,
}

impl<'state> ApplyHost for ManagedHost<'state> {
    type Snapshot = CommittedSnapshot;
    type Access = WriteGuard;
    type Authorization = ();
    type Publication = MutexGuard<'state, ()>;
    type Receipt = Project;

    fn task_key(&self) -> &str {
        self.project_id
    }
    fn project(snapshot: &Self::Snapshot) -> &Project {
        &snapshot.project
    }
    async fn capture(&self) -> Result<(WriteGuard, CommittedSnapshot)> {
        let guard = crate::project_access::ensure_write(self.state, self.project_id)?;
        let snapshot = legacy::capture_with_history(self.state, self.project_id).await?;
        Ok((guard, snapshot))
    }
    async fn authorize(&self) -> Result<()> {
        crate::permissions::consume(self.state, self.project_id, "modify_project").await
    }
    async fn latest<'a>(&'a self, original: &'a CommittedSnapshot) -> Result<CommittedSnapshot> {
        legacy::recheck_lifecycle(self.state, original)?;
        let latest = legacy::capture_with_history(self.state, self.project_id).await?;
        legacy::recheck_lifecycle(self.state, original)?;
        let before = &original.project;
        let after = &latest.project;
        if before.current_revision != after.current_revision
            || before.files != after.files
            || before.revisions != after.revisions
            || original.history != latest.history
        {
            return Err(AppError::Invalid(
                "Project changed during modeling; retry the request".into(),
            ));
        }
        Ok(latest)
    }
    async fn build<'a>(
        &'a self,
        original: &'a CommittedSnapshot,
        request: &'a ApplyRequest,
        prepared: &'a PreparedApply,
        cancel: Arc<AtomicBool>,
    ) -> Result<GeneratedModel> {
        build::build(
            self.state,
            &original.project,
            &request.program,
            prepared.program(),
            request.review_plan.as_ref().zip(prepared.review_document()),
            cancel,
            self.executable,
        )
        .await
    }
    async fn lock_publication(&self) -> Result<MutexGuard<'state, ()>> {
        Ok(self.state.writes.lock().await)
    }
    async fn publish<'a>(
        &'a self,
        original: &'a CommittedSnapshot,
        latest: &'a CommittedSnapshot,
        _authorization: (),
        _guard: MutexGuard<'state, ()>,
        prepared: PreparedModelChange,
    ) -> Result<Project> {
        legacy::recheck_lifecycle(self.state, original)?;
        legacy::recheck(self.state, latest).await?;
        legacy::recheck_lifecycle(self.state, original)?;
        commit::persist_prepared(self.state, prepared).await
    }
}
