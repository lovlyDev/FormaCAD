//! Session-long OS leases. Reading/listing does not acquire write ownership.
pub mod copy;
pub mod ipc;
mod registry;
#[cfg(test)]
mod tests;
pub(crate) mod transient;
use crate::core::{AppState, Result};
pub use registry::{AccessRegistry, AccessStatus, WriteGuard};

pub fn ensure_write(state: &AppState, project_id: &str) -> Result<WriteGuard> {
    state.project_access.ensure_write(&state.root, project_id)
}
