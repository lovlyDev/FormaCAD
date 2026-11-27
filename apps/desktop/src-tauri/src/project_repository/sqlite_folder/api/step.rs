//! Explicit last-STEP transfer host API. No old application database is opened.
use super::super::step_transfer::{self, StepTransferGrant, TransferIntent};
use super::{ModelRuntime, Publication};
use crate::{
    core::Result,
    modeling::{execution::BuildRuntime, repository_apply::ApplyRuntime},
    models::ProjectFile,
    project_repository::selected_step::{SelectedStep, StepSnapshot},
};
use std::path::Path;

pub struct SelectedStepFile {
    source: SelectedStep,
}
pub struct StepRead {
    original: StepSnapshot,
}
impl StepRead {
    pub fn descriptor(&self) -> &ProjectFile {
        self.original.descriptor()
    }
}
/// One trusted host decision for one original source and physical destination.
/// There is no constructor from a UUID, serialized token, path or boolean alone.
pub struct StepDestinationApproval {
    grant: StepTransferGrant,
}
pub struct StepProjectIntent {
    pub name: String,
    pub agent: String,
    pub prompt: String,
}
impl SelectedStepFile {
    pub fn open(path: &Path) -> Result<Self> {
        Ok(Self {
            source: SelectedStep::open(path)?,
        })
    }
    pub fn capture(&self) -> Result<StepRead> {
        Ok(StepRead {
            original: self.source.capture()?,
        })
    }
    pub fn recheck(&self, original: &StepRead) -> Result<()> {
        self.source.recheck(&original.original)
    }
    pub fn read(&self, original: &StepRead) -> Result<Vec<u8>> {
        self.source.read(&original.original)
    }
    pub fn close(&self) {
        self.source.close();
    }
    /// Host confirmation follows the explicit source and destination selections.
    pub fn approve_destination(
        &self,
        original: &StepRead,
        destination: &Path,
    ) -> Result<StepDestinationApproval> {
        Ok(StepDestinationApproval {
            grant: StepTransferGrant::approve(&self.source, &original.original, destination)?,
        })
    }
    pub async fn transfer_to_folder(
        &self,
        original: &StepRead,
        approval: StepDestinationApproval,
        intent: StepProjectIntent,
        runtime: ModelRuntime<'_>,
    ) -> Result<Publication> {
        Ok(step_transfer::transfer(
            &self.source,
            &original.original,
            approval.grant,
            TransferIntent {
                name: intent.name,
                agent: intent.agent,
                prompt: intent.prompt,
            },
            ApplyRuntime::new(runtime.tasks, runtime.gate),
            BuildRuntime {
                runtime_root: runtime.runtime_root,
                transient_root: runtime.transient_root,
                executable: runtime.executable,
            },
        )
        .await?
        .into())
    }
}

#[cfg(test)]
mod tests;
