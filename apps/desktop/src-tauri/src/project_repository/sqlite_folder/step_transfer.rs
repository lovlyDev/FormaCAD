//! A selected STEP creates a new independent project through the common CAD
//! algorithm. No old project database, parametric history or app SQL is adopted.
use super::{creation, format, publication_verification};
use crate::{
    core::{AppError, Result},
    modeling::{
        apply::ApplyRequest,
        execution::{self, BuildRuntime},
        prepared_change::PreparedModelChange,
        prepared_request::PreparedApply,
        repository_apply::{self, ApplyHost, ApplyRuntime, RepositoryApply},
        GeneratedModel,
    },
    models::Project,
    project_repository::selected_step::{SelectedStep, StepSnapshot},
};
use std::sync::{atomic::AtomicBool, Arc, Mutex};

mod authorization;
pub(super) use authorization::StepTransferGrant;

pub(super) struct TransferIntent {
    pub name: String,
    pub agent: String,
    pub prompt: String,
}

struct TransferSnapshot {
    source: StepSnapshot,
    project: Project,
}
struct TransferHost<'a> {
    source: &'a SelectedStep,
    expected: &'a StepSnapshot,
    project: Project,
    destination: Arc<authorization::Destination>,
    grant: Mutex<Option<StepTransferGrant>>,
    runtime: BuildRuntime<'a>,
}
fn changed() -> AppError {
    AppError::Invalid("PROJECT_SNAPSHOT_CHANGED".into())
}

/// The target's first SQL commit contains the validated imported model. Before
/// commit, errors may leave only owned unreferenced CAS staging, never a model
/// success. After commit, errors retain the known UUID without replay/deletion.
pub(super) async fn transfer(
    source: &SelectedStep,
    original: &StepSnapshot,
    grant: StepTransferGrant,
    intent: TransferIntent,
    services: ApplyRuntime<'_>,
    runtime: BuildRuntime<'_>,
) -> Result<format::CommitOutcome> {
    grant.check(source, original)?;
    grant.destination.empty()?;
    let now = chrono::Utc::now().to_rfc3339();
    let project = Project {
        schema_version: 1,
        id: uuid::Uuid::new_v4().to_string(),
        name: intent.name,
        units: "mm".into(),
        agent: intent.agent,
        pinned: false,
        thumbnail: None,
        thumbnail_revision: None,
        created_at: now.clone(),
        updated_at: now,
        revisions: vec![],
        current_revision: None,
        messages: vec![],
        files: vec![original.descriptor().clone()],
        exports: vec![],
    };
    project.validate()?;
    let bytes = source.read(original)?;
    let program = crate::conversion::import_document::source(
        &uuid::Uuid::new_v4().to_string(),
        &original.descriptor().name,
        &bytes,
    )?;
    drop(bytes);
    grant.check(source, original)?;
    let request = ApplyRequest::new(project.id.clone(), program, intent.prompt, None);
    let host = TransferHost {
        source,
        expected: original,
        project,
        destination: Arc::clone(&grant.destination),
        grant: Mutex::new(Some(grant)),
        runtime,
    };
    match repository_apply::apply(services, &host, request).await? {
        RepositoryApply::Published(outcome) => Ok(outcome),
        RepositoryApply::NoOp(_) => Err(changed()),
    }
}

impl ApplyHost for TransferHost<'_> {
    type Snapshot = TransferSnapshot;
    type Access = ();
    type Authorization = StepTransferGrant;
    type Publication = ();
    type Receipt = format::CommitOutcome;

    fn task_key(&self) -> &str {
        self.expected.task_key()
    }
    fn project(snapshot: &TransferSnapshot) -> &Project {
        &snapshot.project
    }
    async fn capture(&self) -> Result<((), TransferSnapshot)> {
        self.source.recheck(self.expected)?;
        self.destination.empty()?;
        // Retain the actual original opening/hash, never recapture a new source.
        let source = self.expected.clone();
        Ok((
            (),
            TransferSnapshot {
                source,
                project: self.project.clone(),
            },
        ))
    }
    async fn authorize(&self) -> Result<StepTransferGrant> {
        let grant = self
            .grant
            .lock()
            .map_err(|_| changed())?
            .take()
            .ok_or_else(|| AppError::Invalid("PERMISSION_REQUIRED".into()))?;
        grant.check(self.source, self.expected)?;
        Ok(grant)
    }
    async fn latest<'a>(&'a self, original: &'a TransferSnapshot) -> Result<TransferSnapshot> {
        self.source.recheck(&original.source)?;
        self.destination.empty()?;
        Ok(TransferSnapshot {
            source: original.source.clone(),
            project: original.project.clone(),
        })
    }
    async fn build<'a>(
        &'a self,
        original: &'a TransferSnapshot,
        request: &'a ApplyRequest,
        prepared: &'a PreparedApply,
        cancel: Arc<AtomicBool>,
    ) -> Result<GeneratedModel> {
        self.source.recheck(&original.source)?;
        let mut assets = |file: &crate::models::ProjectFile| {
            if file != original.source.descriptor() {
                return Err(changed());
            }
            self.source.read(&original.source)
        };
        execution::build(
            BuildRuntime {
                runtime_root: self.runtime.runtime_root,
                transient_root: self.runtime.transient_root,
                executable: self.runtime.executable,
            },
            &mut assets,
            &original.project,
            &request.program,
            prepared.program(),
            None,
            cancel,
        )
        .await
    }
    async fn lock_publication(&self) -> Result<()> {
        self.destination.check()
    }
    async fn publish<'a>(
        &'a self,
        original: &'a TransferSnapshot,
        latest: &'a TransferSnapshot,
        grant: StepTransferGrant,
        _guard: (),
        prepared: PreparedModelChange,
    ) -> Result<format::CommitOutcome> {
        if !Arc::ptr_eq(&grant.destination, &self.destination) {
            return Err(changed());
        }
        grant.check(self.source, &original.source)?;
        self.source.recheck(&latest.source)?;
        self.destination.empty()?;
        let (project, mut pending) = prepared.into_parts();
        let hash = original
            .source
            .descriptor()
            .sha256
            .as_ref()
            .ok_or_else(changed)?;
        if !pending.iter().any(|(saved, _)| saved == hash) {
            pending.push((hash.clone(), self.source.read(&original.source)?));
        }
        let outcome =
            creation::create_checked(&self.destination.root, project, pending, None, || async {
                grant.check(self.source, &original.source)
            })
            .await?;
        match outcome {
            format::CommitOutcome::Committed(saved) => {
                let verified = async {
                    grant.check(self.source, &original.source)?;
                    self.destination.verify(&saved).await?;
                    grant.check(self.source, &original.source)
                }
                .await;
                match verified {
                    Ok(()) => Ok(format::CommitOutcome::Committed(saved)),
                    Err(error) => Ok(format::CommitOutcome::Indeterminate {
                        commit_id: saved.expectation().commit_id().to_owned(),
                        error,
                    }),
                }
            }
            outcome => Ok(outcome),
        }
    }
}

#[cfg(test)]
mod tests;
