//! Host-only, one-use decisions retaining the original source and typed command.
//! No renderer boolean, source selector, audit row or persisted setting is a grant.
use super::{operation::FolderOperation, WindowSources};
use crate::{
    core::{AppError, Result},
    project_history::Direction,
    project_repository::sqlite_folder::api::{
        ModelChange, ModelIntent, ModelRuntime, WriteApproval,
    },
};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

const LIFETIME: Duration = Duration::from_secs(300);
const LIMIT: usize = 128;

/// Selectors are checked against the window registry before an operation is kept.
#[derive(Clone, Copy)]
pub struct ApprovalSource<'a> {
    pub window: &'a str,
    pub source_id: &'a str,
    pub project_id: &'a str,
    pub commit_id: &'a str,
}

/// The exact command is retained before the decision, never supplied at consume.
pub enum WriteIntent {
    Model(ModelIntent),
    History(Direction),
}

struct Command {
    operation: FolderOperation,
    intent: WriteIntent,
}
enum Decision {
    Pending(Command),
    Allowed(Command),
}
struct Request {
    window: String,
    source_id: String,
    project_id: String,
    commit_id: String,
    expires: Instant,
    decision: Mutex<Option<Decision>>,
}

/// Owned by WindowSources; map locks never span source IO or an await.
#[derive(Default)]
pub(super) struct ApprovalRequests(std::sync::Mutex<HashMap<String, Arc<Request>>>);

fn denied() -> AppError {
    AppError::Permission("Request expired or was already used".into())
}
impl Request {
    fn matches(&self, source: ApprovalSource<'_>) -> bool {
        self.window == source.window
            && self.source_id == source.source_id
            && self.project_id == source.project_id
            && self.commit_id == source.commit_id
    }
    fn live(&self) -> Result<()> {
        if Instant::now() >= self.expires {
            Err(denied())
        } else {
            Ok(())
        }
    }
}
impl ApprovalRequests {
    fn get(&self, id: &str) -> Result<Arc<Request>> {
        let mut requests = self.0.lock().map_err(|_| denied())?;
        requests.retain(|_, request| request.expires > Instant::now());
        requests.get(id).cloned().ok_or_else(denied)
    }
    fn remove(&self, id: &str) {
        if let Ok(mut requests) = self.0.lock() {
            requests.remove(id);
        }
    }
    pub(super) fn close(&self, window: &str, source: Option<&str>) {
        if let Ok(mut requests) = self.0.lock() {
            requests.retain(|_, request| {
                request.window != window || source.is_some_and(|source| request.source_id != source)
            });
        }
    }
}

/// Private fields prevent substituting a different command after approval.
/// The publisher still checks lifecycle, saved state and the actual OS lease.
pub struct ApprovedCommand {
    operation: FolderOperation,
    intent: WriteIntent,
    approval: WriteApproval,
}
impl ApprovedCommand {
    pub async fn execute(self, runtime: ModelRuntime<'_>) -> Result<ModelChange> {
        match self.intent {
            WriteIntent::Model(intent) => {
                self.operation
                    .apply_model(Some(self.approval), intent, runtime)
                    .await
            }
            WriteIntent::History(direction) => self
                .operation
                .apply_history(self.approval, direction)
                .await
                .map(ModelChange::Published),
        }
    }
}

impl WindowSources {
    pub async fn request_write(
        &self,
        source: ApprovalSource<'_>,
        intent: WriteIntent,
    ) -> Result<String> {
        self.request_prepared_write(source, |_| Ok(intent)).await
    }

    /// Prepare and validate the typed host command against this exact original
    /// project. The callback must be pure; no second source lookup/capture is needed.
    pub async fn request_prepared_write(
        &self,
        source: ApprovalSource<'_>,
        prepare: impl FnOnce(&crate::models::Project) -> Result<WriteIntent> + Send,
    ) -> Result<String> {
        let operation = self
            .capture_operation(
                source.window,
                source.source_id,
                source.project_id,
                source.commit_id,
            )
            .await?;
        operation.recheck().await?;
        let intent = prepare(operation.project())?;
        let request = Arc::new(Request {
            window: source.window.into(),
            source_id: source.source_id.into(),
            project_id: source.project_id.into(),
            commit_id: source.commit_id.into(),
            expires: Instant::now() + LIFETIME,
            decision: Mutex::new(Some(Decision::Pending(Command { operation, intent }))),
        });
        let id = uuid::Uuid::new_v4().to_string();
        let mut requests = self.approvals.0.lock().map_err(|_| denied())?;
        requests.retain(|_, request| request.expires > Instant::now());
        if requests.len() >= LIMIT {
            return Err(denied());
        }
        requests.insert(id.clone(), request);
        Ok(id)
    }

    /// Only the trusted existing permission host may call this after its decision.
    /// A per-request lock serializes resolve/consume without blocking other grants.
    pub async fn resolve_write(&self, id: &str, window: &str, allow: bool) -> Result<()> {
        let request = self.approvals.get(id)?;
        if request.window != window {
            return Err(denied());
        }
        let mut decision = request.decision.lock().await;
        request.live()?;
        if !matches!(*decision, Some(Decision::Pending(_))) {
            return Err(denied());
        }
        let Some(Decision::Pending(command)) = decision.take() else {
            unreachable!()
        };
        if !allow {
            self.approvals.remove(id);
            return Ok(());
        }
        // On stale/error the taken request stays terminal, never pending again.
        command.operation.recheck().await?;
        request.live()?;
        *decision = Some(Decision::Allowed(command));
        Ok(())
    }

    pub async fn consume_write(
        &self,
        id: &str,
        source: ApprovalSource<'_>,
    ) -> Result<ApprovedCommand> {
        let request = self.approvals.get(id)?;
        if !request.matches(source) {
            return Err(denied());
        }
        let mut decision = request.decision.lock().await;
        request.live()?;
        if !matches!(*decision, Some(Decision::Allowed(_))) {
            return Err(denied());
        }
        let Some(Decision::Allowed(command)) = decision.take() else {
            unreachable!()
        };
        self.approvals.remove(id);
        let approval = command.operation.approve_write().await?;
        request.live()?;
        Ok(ApprovedCommand {
            operation: command.operation,
            intent: command.intent,
            approval,
        })
    }
}

#[cfg(test)]
mod tests;
