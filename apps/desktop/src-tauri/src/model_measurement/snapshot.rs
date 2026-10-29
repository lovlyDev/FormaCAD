//! Compatibility boundary for the shared, pure committed snapshot reader.
use super::failure;
use crate::{
    core::{AppError, AppState, Result},
    project_repository::legacy::{self, CommittedSnapshot},
};

pub(super) async fn read(state: &AppState, project_id: &str) -> Result<CommittedSnapshot> {
    legacy::capture_project(state, project_id)
        .await
        .map_err(compatible)
}

pub(super) async fn recheck(state: &AppState, snapshot: &CommittedSnapshot) -> Result<()> {
    legacy::recheck(state, snapshot).await.map_err(compatible)
}

fn compatible(error: AppError) -> AppError {
    match error {
        AppError::Invalid(ref code)
            if code == "PROJECT_SNAPSHOT_MISSING" || code == "PROJECT_SNAPSHOT_CHANGED" =>
        {
            failure(
                "MEASUREMENT_STALE",
                "Measurement committed project snapshot changed",
            )
        }
        AppError::Invalid(ref code) if code == "PROJECT_SNAPSHOT_RECOVERY_REQUIRED" => failure(
            "MEASUREMENT_RECOVERY_REQUIRED",
            "Project storage recovery must finish before exact measurement; reopen the project",
        ),
        other => other,
    }
}
