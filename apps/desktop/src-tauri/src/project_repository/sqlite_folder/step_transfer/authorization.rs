//! Explicit one-use host decision for one selected source and destination.
use super::*;
use std::sync::Arc;

pub(super) struct Destination {
    pub(super) root: std::path::PathBuf,
    location: crate::project_repository::location::LocationIdentity,
}
impl Destination {
    pub(super) fn check(&self) -> Result<()> {
        publication_verification::recheck_location(&self.location, &self.root)
    }
    pub(super) fn empty(&self) -> Result<()> {
        self.check()?;
        if let Some(entry) = std::fs::read_dir(&self.root)?.next() {
            entry?;
            return Err(changed());
        }
        self.check()
    }
    pub(super) async fn verify(&self, saved: &format::Snapshot) -> Result<()> {
        publication_verification::verify(&self.root, &self.location, saved).await?;
        self.check()
    }
}

pub(in crate::project_repository::sqlite_folder) struct StepTransferGrant {
    source_opening: String,
    pub(super) destination: Arc<Destination>,
}
impl StepTransferGrant {
    /// Trusted host entry after the explicit selection/creation decision. This
    /// value is never deserialized or reconstructed from a project UUID.
    pub(in crate::project_repository::sqlite_folder) fn approve(
        source: &SelectedStep,
        original: &StepSnapshot,
        target: &std::path::Path,
    ) -> Result<Self> {
        source.recheck(original)?;
        if !target.is_absolute() {
            return Err(changed());
        }
        let location = crate::project_repository::location::LocationIdentity::capture(
            target,
            std::path::Path::new(""),
        )?;
        let destination = Arc::new(Destination {
            root: target.into(),
            location,
        });
        destination.empty()?;
        source.recheck(original)?;
        Ok(Self {
            source_opening: original.task_key().into(),
            destination,
        })
    }
    pub(super) fn check(&self, source: &SelectedStep, original: &StepSnapshot) -> Result<()> {
        if self.source_opening != original.task_key() {
            return Err(changed());
        }
        source.recheck(original)?;
        self.destination.check()
    }
}
