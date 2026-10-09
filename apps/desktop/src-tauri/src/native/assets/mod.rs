//! Project-scoped host staging and independently verified worker STEP inputs.
mod resolve;
mod stage;
pub use resolve::read_step;
pub use stage::stage_project_assets;

pub const MAX_TOTAL_STEP_BYTES: usize = 128 * 1024 * 1024;
/// Check every authored input before producing any geometry/artifact.
pub fn verify_inputs(
    document: &crate::cad_ir::Document,
    cwd: &std::path::Path,
) -> Result<(), super::NativeError> {
    let mut total = 0usize;
    let references = crate::cad_ir::assets::references(document)
        .map_err(|e| error("ASSET_INVALID", e.to_string()))?;
    for (id, hash) in references {
        let (_, size) = resolve::checked_path(cwd, id, hash)?;
        total += size;
        if total > MAX_TOTAL_STEP_BYTES {
            return Err(error("ASSET_LIMIT", "Total STEP input exceeds 128 MB"));
        }
    }
    Ok(())
}
pub fn relative_path(sha256: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(format!("inputs/step-{sha256}.step"))
}
pub(super) fn error(code: &str, detail: impl Into<String>) -> super::NativeError {
    super::NativeError {
        code: code.into(),
        detail: detail.into(),
    }
}
pub(super) fn app_error(code: &str, detail: impl Into<String>) -> crate::core::AppError {
    crate::core::AppError::Invalid(
        serde_json::json!({"code":code,"message":detail.into()}).to_string(),
    )
}
