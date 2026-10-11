//! Revalidate original project imported STEP bytes, rather than worker copies.
use crate::{
    cad_ir::Document,
    core::{AppError, AppState, Result},
    models::Project,
};
pub(super) fn verify(state: &AppState, project: &Project, document: &Document) -> Result<()> {
    let mut total = 0usize;
    for (_, hash) in
        crate::cad_ir::assets::references(document).map_err(crate::cad_ir::app_error)?
    {
        let file = project
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
        let bytes = crate::artifacts::read(state, &project.id, file)?;
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
    Ok(())
}
