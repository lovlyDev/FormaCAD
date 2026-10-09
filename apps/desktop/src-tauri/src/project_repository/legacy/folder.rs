use super::changed;
use crate::{
    core::{AppState, Result},
    models::Project,
};
use std::path::Path;
pub(super) fn check(
    state: &AppState,
    project: &mut Project,
    history: Option<&[u8]>,
) -> Result<Option<String>> {
    let root = crate::security::guarded(&state.root, Path::new(&project.id))?;
    if !root.join("manifest.json").exists() {
        return Ok(None);
    }
    if project.files.iter().any(|file| file.data.is_some()) {
        // Memory only: never materialize the normalized artifact bytes.
        let _pending = crate::artifacts::normalize(project)?;
    }
    let (disk, seal) = crate::project_storage_v2::inspect_folder(&root)?;
    if serde_json::to_vec(&disk)? != serde_json::to_vec(project)? {
        return Err(changed());
    }
    if let Some(expected) = history {
        if let Some(bytes) = crate::project_storage_v2::read_history(&root)? {
            crate::project_history::validate_archive(&disk, &bytes)?;
            if bytes != expected {
                return Err(changed());
            }
        }
        let (_, latest) = crate::project_storage_v2::inspect_folder(&root)?;
        if latest != seal {
            return Err(changed());
        }
    }
    *project = disk;
    Ok(Some(seal))
}
