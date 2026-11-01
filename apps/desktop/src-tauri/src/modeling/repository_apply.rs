//! One modeling algorithm. Hosts retain source authority and durable publication.
use super::{
    apply::ApplyRequest,
    prepared_change::{self, PreparedModelChange},
    prepared_request::{self, PreparedApply, PreparedRequest},
    GeneratedModel,
};
use crate::{
    cad_tasks::CadTaskGate,
    core::{AppError, AppState, Result},
    models::Project,
};
use std::{
    collections::HashMap,
    future::Future,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tokio::sync::Mutex;

pub(super) mod managed;

/// Runtime services only: no project SQL, paths, grants or project access registry.
pub(crate) struct ApplyRuntime<'a> {
    tasks: &'a Mutex<HashMap<String, Arc<AtomicBool>>>,
    gate: &'a CadTaskGate,
}
impl<'a> ApplyRuntime<'a> {
    pub(crate) fn new(
        tasks: &'a Mutex<HashMap<String, Arc<AtomicBool>>>,
        gate: &'a CadTaskGate,
    ) -> Self {
        Self { tasks, gate }
    }

    pub(crate) fn from_state(state: &'a AppState) -> Self {
        Self::new(&state.tasks, &state.cad_tasks)
    }
}

/// A publication receipt is backend-specific. Published never means confirmed
/// success by itself: a folder receipt can retain an indeterminate commit UUID.
pub(crate) enum RepositoryApply<R> {
    NoOp(Project),
    Published(R),
}

/// Static dispatch; every asynchronous operation promises Send. An owned host
/// may contain Arc<FolderSession>; snapshots/authorization cannot be IPC data.
pub(crate) trait ApplyHost: Sync {
    type Snapshot: Send + Sync;
    type Access: Send;
    type Authorization: Send;
    type Publication<'a>: Send
    where
        Self: 'a;
    type Receipt: Send;

    /// Managed project ID or an opening-specific UUID, never a folder project ID.
    fn task_key(&self) -> &str;
    fn project(snapshot: &Self::Snapshot) -> &Project;
    fn capture(&self) -> impl Future<Output = Result<(Self::Access, Self::Snapshot)>> + Send;
    fn authorize(&self) -> impl Future<Output = Result<Self::Authorization>> + Send;
    /// Recheck original source lifecycle and model/history seals; retain latest metadata.
    fn latest<'a>(
        &'a self,
        original: &'a Self::Snapshot,
    ) -> impl Future<Output = Result<Self::Snapshot>> + Send + 'a;
    /// Delegate to the existing executor. Folder adapters supply their own bounded
    /// captured asset reader; managed keeps its existing build wrapper/classification.
    fn build<'a>(
        &'a self,
        original: &'a Self::Snapshot,
        request: &'a ApplyRequest,
        prepared: &'a PreparedApply,
        cancel: Arc<AtomicBool>,
    ) -> impl Future<Output = Result<GeneratedModel>> + Send + 'a;
    fn lock_publication(&self) -> impl Future<Output = Result<Self::Publication<'_>>> + Send;
    /// Consume genuine backend authority and the common prepared candidate.
    /// Both source snapshots survive through the actual durable commit.
    fn publish<'a>(
        &'a self,
        original: &'a Self::Snapshot,
        latest: &'a Self::Snapshot,
        authorization: Self::Authorization,
        guard: Self::Publication<'a>,
        prepared: PreparedModelChange,
    ) -> impl Future<Output = Result<Self::Receipt>> + Send + 'a;
}

fn cancelled(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Relaxed) {
        Err(AppError::Invalid("CAD_TASK_CANCELLED".into()))
    } else {
        Ok(())
    }
}

pub(crate) async fn apply<H: ApplyHost>(
    runtime: ApplyRuntime<'_>,
    host: &H,
    request: ApplyRequest,
) -> Result<RepositoryApply<H::Receipt>> {
    let (_access, original) = host.capture().await?;
    if H::project(&original).id != request.project_id {
        return Err(AppError::Invalid("PROJECT_SNAPSHOT_CHANGED".into()));
    }
    let prepared = match prepared_request::prepare(H::project(&original), &request)? {
        PreparedRequest::NoOp => return Ok(RepositoryApply::NoOp(H::project(&original).clone())),
        PreparedRequest::Apply(prepared) => *prepared,
    };
    let authorization = host.authorize().await?;
    let key = host.task_key();
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut tasks = runtime.tasks.lock().await;
        if tasks.contains_key(key) {
            return Err(AppError::Invalid(
                "A project task is already running".into(),
            ));
        }
        tasks.insert(key.into(), cancel.clone());
    }
    let result = async {
        let permit = runtime
            .gate
            .acquire(key, "model_build", cancel.clone())
            .await?;
        host.latest(&original).await?;
        cancelled(&cancel)?;
        let generated = host
            .build(&original, &request, &prepared, cancel.clone())
            .await?;
        drop(permit);
        cancelled(&cancel)?;
        let guard = host.lock_publication().await?;
        cancelled(&cancel)?;
        let latest = host.latest(&original).await?;
        let (job, saved_program) = prepared.into_commit_parts();
        let candidate = prepared_change::prepare(
            H::project(&latest).clone(),
            generated,
            job,
            saved_program,
            request.prompt,
            chrono::Utc::now().to_rfc3339(),
            &cancel,
        )?;
        cancelled(&cancel)?;
        let receipt = host
            .publish(&original, &latest, authorization, guard, candidate)
            .await?;
        Ok(RepositoryApply::Published(receipt))
    }
    .await;
    // Preserve the current ordinary Result cleanup contract. This is NOT
    // abort/panic cleanup; dropping a future is not proof of publication rollback.
    let mut tasks = runtime.tasks.lock().await;
    if tasks
        .get(key)
        .is_some_and(|registered| Arc::ptr_eq(registered, &cancel))
    {
        tasks.remove(key);
    }
    result
}

#[cfg(test)]
mod tests;
