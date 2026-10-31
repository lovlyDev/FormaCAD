//! Internal read identity. A lifecycle token is not an OS lease or a write capability.
use super::location::LocationIdentity;
use crate::core::{AppState, Result};
use std::path::Path;

#[derive(Debug, PartialEq, Eq)]
enum Backend {
    LegacyManaged,
}

/// Bound to one application registry, managed location and project lifecycle.
/// Consumers must also revalidate their captured source before returning results.
pub(crate) struct ReadSession {
    backend: Backend,
    project_id: String,
    location: LocationIdentity,
    epoch: uuid::Uuid,
}

impl ReadSession {
    pub(crate) fn capture(state: &AppState, project_id: &str) -> Result<Self> {
        crate::security::valid_id(project_id)?;
        Ok(Self {
            backend: Backend::LegacyManaged,
            project_id: project_id.into(),
            location: LocationIdentity::capture(&state.root, Path::new(project_id))?,
            epoch: state.project_access.read_epoch(project_id)?,
        })
    }

    pub(crate) fn recheck(&self, state: &AppState, project_id: &str) -> Result<()> {
        if self.project_id != project_id
            || self.backend != Backend::LegacyManaged
            || self.epoch != state.project_access.read_epoch(project_id)?
            || !self.location.recheck(&state.root, Path::new(project_id))?
        {
            return Err(super::legacy::changed());
        }
        Ok(())
    }
}
