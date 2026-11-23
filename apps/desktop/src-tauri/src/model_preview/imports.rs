//! Revalidate imported STEP through the original repository source.
use crate::{
    cad_ir::Document,
    core::{AppError, Result},
};
pub(super) async fn verify_read(
    read: &crate::project_repository::backend::RepositoryRead<'_>,
    document: &Document,
) -> Result<()> {
    let mut total = 0usize;
    for (_, hash) in
        crate::cad_ir::assets::references(document).map_err(crate::cad_ir::app_error)?
    {
        let file = read
            .project()
            .files
            .iter()
            .find(|file| {
                file.sha256.as_deref() == Some(hash)
                    && std::path::Path::new(&file.name)
                        .extension()
                        .and_then(|v| v.to_str())
                        .is_some_and(|v| {
                            v.eq_ignore_ascii_case("step") || v.eq_ignore_ascii_case("stp")
                        })
            })
            .ok_or_else(|| AppError::Invalid("PROJECT_SNAPSHOT_CHANGED".into()))?;
        let bytes = read.read_asset(file).await?;
        total = total
            .checked_add(bytes.len())
            .ok_or_else(|| AppError::Invalid("PROJECT_SNAPSHOT_CHANGED".into()))?;
        if bytes.is_empty()
            || bytes.len() as u64 != file.size
            || crate::artifacts::digest(&bytes) != hash
            || total > crate::native::assets::MAX_TOTAL_STEP_BYTES
        {
            return Err(AppError::Invalid("PROJECT_SNAPSHOT_CHANGED".into()));
        }
    }
    read.recheck().await
}

pub(super) async fn stage_read(
    read: &crate::project_repository::backend::RepositoryRead<'_>,
    document: &Document,
    cwd: &std::path::Path,
) -> Result<()> {
    crate::model_measurement::staging::stage_assets(read, document, cwd)
        .await
        .map_err(|error| {
            let code = match &error {
                AppError::Invalid(text) => serde_json::from_str::<serde_json::Value>(text)
                    .ok()
                    .and_then(|v| v.get("code").and_then(|v| v.as_str()).map(str::to_owned)),
                _ => None,
            };
            match code.as_deref() {
                Some("MEASUREMENT_STALE") => AppError::Invalid("PROJECT_SNAPSHOT_CHANGED".into()),
                Some("MEASUREMENT_RECOVERY_REQUIRED") => {
                    AppError::Invalid("PROJECT_SNAPSHOT_RECOVERY_REQUIRED".into())
                }
                _ => error,
            }
        })
}
