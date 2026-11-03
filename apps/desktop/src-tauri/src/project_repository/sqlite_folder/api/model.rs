//! Host-owned CAD services use the one existing algorithm, not a second model builder.
use super::super::session::apply::FolderApplyHost;
use super::{FolderProject, Publication, ReadView, WriteApproval};
use crate::{
    cad_tasks::CadTaskGate,
    core::Result,
    modeling::{
        apply::ApplyRequest,
        execution::BuildRuntime,
        repository_apply::{self, ApplyRuntime, RepositoryApply},
    },
    models::Project,
};
use std::{
    collections::HashMap,
    path::Path,
    sync::{atomic::AtomicBool, Arc},
};
use tokio::sync::Mutex;

/// Trusted host services and paths. Never construct this value from an IPC request.
pub struct ModelRuntime<'a> {
    pub tasks: &'a Mutex<HashMap<String, Arc<AtomicBool>>>,
    pub gate: &'a CadTaskGate,
    pub runtime_root: &'a Path,
    pub transient_root: &'a Path,
    pub executable: Option<&'a Path>,
}

pub struct ModelIntent {
    pub program: String,
    pub prompt: String,
}

pub enum ModelChange {
    NoOp(Box<Project>),
    Published(Publication),
}

impl FolderProject {
    /// The caller's original view determines project ID and expected revision.
    /// A newer capture can never replace the state approved by the caller.
    pub async fn apply_model(
        &self,
        original: &ReadView,
        approval: Option<WriteApproval>,
        intent: ModelIntent,
        runtime: ModelRuntime<'_>,
    ) -> Result<ModelChange> {
        if let Some(approval) = &approval {
            approval.check(self, original)?;
        }
        self.recheck(original).await?;
        let request = ApplyRequest::new(
            original.project().id.clone(),
            intent.program,
            intent.prompt,
            original.project().current_revision.clone(),
        );
        let host = FolderApplyHost::new(
            &self.session,
            BuildRuntime {
                runtime_root: runtime.runtime_root,
                transient_root: runtime.transient_root,
                executable: runtime.executable,
            },
            approval.map(|approval| approval.authorization),
        );
        Ok(
            match repository_apply::apply(
                ApplyRuntime::new(runtime.tasks, runtime.gate),
                &host,
                request,
            )
            .await?
            {
                RepositoryApply::NoOp(project) => ModelChange::NoOp(project),
                RepositoryApply::Published(outcome) => ModelChange::Published(outcome.into()),
            },
        )
    }
}
