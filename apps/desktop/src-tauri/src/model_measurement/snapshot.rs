//! Measurement error compatibility over the shared original-source reader.
use super::failure;
use crate::{
    core::{AppError, AppState, Result},
    project_repository::backend::{ProjectRepository, RepositoryRead},
};

pub(super) async fn read<'a>(state: &'a AppState, project_id: &str) -> Result<RepositoryRead<'a>> {
    RepositoryRead::capture(ProjectRepository::managed(state, project_id)?)
        .await
        .map_err(compatible)
}
pub(super) async fn recheck(snapshot: &RepositoryRead<'_>) -> Result<()> {
    snapshot.recheck().await.map_err(compatible)
}
pub(super) async fn recheck_lifecycle(snapshot: &RepositoryRead<'_>) -> Result<()> {
    snapshot.recheck_source().await.map_err(compatible)
}
pub(super) async fn refresh<'a>(snapshot: &RepositoryRead<'a>) -> Result<RepositoryRead<'a>> {
    snapshot.recapture_metadata().await.map_err(compatible)
}
pub(super) fn asset_error(error: AppError) -> AppError {
    match error {
        AppError::Invalid(ref code)
            if matches!(
                code.as_str(),
                "PROJECT_SNAPSHOT_MISSING"
                    | "PROJECT_SNAPSHOT_CHANGED"
                    | "PROJECT_SNAPSHOT_RECOVERY_REQUIRED"
            ) =>
        {
            compatible(error)
        }
        other => failure("ASSET_CHECKSUM_MISMATCH", other.to_string()),
    }
}
pub(super) fn compatible(error: AppError) -> AppError {
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
