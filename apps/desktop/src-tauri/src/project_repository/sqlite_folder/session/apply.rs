//! Folder host for the existing common CAD algorithm. No managed project row.
use super::super::format;
use super::{
    changed, execution_assets::ExecutionAssets, FolderSession, FolderSnapshot, ReadBinding,
};
use crate::{
    core::Result,
    modeling::{
        apply::ApplyRequest,
        execution::{self, BuildRuntime},
        prepared_change::PreparedModelChange,
        prepared_request::PreparedApply,
        repository_apply::ApplyHost,
        GeneratedModel,
    },
    models::{Project, ProjectFile},
};
use std::sync::{atomic::AtomicBool, Arc, Mutex};

mod authorization;
pub(in crate::project_repository::sqlite_folder) use authorization::FolderAuthorization;

pub(in crate::project_repository::sqlite_folder) struct FolderApplyHost<'a> {
    session: &'a FolderSession,
    runtime: BuildRuntime<'a>,
    authorization: Mutex<Option<FolderAuthorization>>,
}

impl<'a> FolderApplyHost<'a> {
    /// The caller supplies trusted runtime paths and an explicit host grant.
    /// None permits the common no-op path but cannot execute a model change.
    pub(in crate::project_repository::sqlite_folder) fn new(
        session: &'a FolderSession,
        runtime: BuildRuntime<'a>,
        authorization: Option<FolderAuthorization>,
    ) -> Self {
        Self {
            session,
            runtime,
            authorization: Mutex::new(authorization),
        }
    }
}

// Select inputs only; the existing executor/stager retains validation and error
// classification. A missing base/reference is not silently replaced by another.
fn requested_assets(
    project: &Project,
    request: &ApplyRequest,
    prepared: &PreparedApply,
) -> Result<Vec<ProjectFile>> {
    if !prepared.program().typed {
        let name = project
            .revisions
            .iter()
            .find(|revision| Some(&revision.id) == project.current_revision.as_ref())
            .and_then(|revision| revision.program_base.as_ref().or(revision.source.as_ref()))
            .filter(|name| is_step(name));
        return Ok(project
            .files
            .iter()
            .filter(|file| Some(&file.name) == name)
            .cloned()
            .collect());
    }
    #[cfg(feature = "native-worker-client")]
    {
        let value: serde_json::Value = serde_json::from_str(&request.program)?;
        if value["schemaVersion"] == 2 {
            let document: crate::cad_ir::Document = serde_json::from_value(value)?;
            let references =
                crate::cad_ir::assets::references(&document).map_err(crate::cad_ir::app_error)?;
            let mut first_by_hash = std::collections::HashMap::new();
            for file in &project.files {
                if is_step(&file.name) {
                    if let Some(hash) = file.sha256.as_deref() {
                        first_by_hash.entry(hash).or_insert(file);
                    }
                }
            }
            return Ok(references
                .into_iter()
                .filter_map(|(_, hash)| first_by_hash.get(hash).copied())
                .cloned()
                .collect());
        }
    }
    #[cfg(not(feature = "native-worker-client"))]
    let _ = request;
    Ok(vec![])
}

fn is_step(name: &str) -> bool {
    std::path::Path::new(name)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("step") || extension.eq_ignore_ascii_case("stp")
        })
}

impl ApplyHost for FolderApplyHost<'_> {
    type Snapshot = FolderSnapshot;
    type Access = ReadBinding;
    type Authorization = FolderAuthorization;
    type Publication = ();
    type Receipt = format::CommitOutcome;

    fn task_key(&self) -> &str {
        &self.session.opening.task_id
    }
    fn project(snapshot: &FolderSnapshot) -> &Project {
        snapshot.project()
    }
    async fn capture(&self) -> Result<(ReadBinding, FolderSnapshot)> {
        let snapshot = self.session.capture().await?;
        {
            let authorization = self.authorization.lock().map_err(|_| changed())?;
            if let Some(grant) = authorization.as_ref() {
                grant.check_capture(self.session, &snapshot)?;
            }
        }
        let access = self.session.read_binding(&snapshot).await?;
        access.check_active()?;
        Ok((access, snapshot))
    }
    async fn authorize(&self) -> Result<FolderAuthorization> {
        let grant = self
            .authorization
            .lock()
            .map_err(|_| changed())?
            .take()
            .ok_or_else(authorization::required)?;
        grant.check(self.session)?;
        Ok(grant)
    }
    async fn latest<'a>(&'a self, original: &'a FolderSnapshot) -> Result<FolderSnapshot> {
        self.session.latest_compatible(original).await
    }
    async fn build<'a>(
        &'a self,
        original: &'a FolderSnapshot,
        request: &'a ApplyRequest,
        prepared: &'a PreparedApply,
        cancel: Arc<AtomicBool>,
    ) -> Result<GeneratedModel> {
        let selected = requested_assets(original.project(), request, prepared)?;
        let mut assets = ExecutionAssets::prefetch(self.session, original, &selected).await?;
        execution::build(
            BuildRuntime {
                runtime_root: self.runtime.runtime_root,
                transient_root: self.runtime.transient_root,
                executable: self.runtime.executable,
            },
            &mut assets,
            original.project(),
            &request.program,
            prepared.program(),
            request.review_plan.as_ref().zip(prepared.review_document()),
            cancel,
        )
        .await
    }
    async fn lock_publication(&self) -> Result<()> {
        self.session.check()
    }
    async fn publish<'a>(
        &'a self,
        original: &'a FolderSnapshot,
        latest: &'a FolderSnapshot,
        authorization: FolderAuthorization,
        _guard: (),
        prepared: PreparedModelChange,
    ) -> Result<format::CommitOutcome> {
        authorization.check(self.session)?;
        let compatible = self.session.latest_compatible(original).await?;
        if !latest.saved.same_saved_state(&compatible.saved) {
            return Err(changed());
        }
        authorization.check(self.session)?;
        let (candidate, pending) = prepared.into_parts();
        self.session.commit(latest, candidate, pending, None).await
    }
}

#[cfg(test)]
mod tests;
