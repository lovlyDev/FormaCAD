//! Verify all saved geometry dependencies before consuming an undo/redo grant.
use crate::{
    core::{AppError, AppState, Result},
    models::Project,
};
use std::collections::HashSet;

pub(super) fn validate_snapshot(state: &AppState, project: &Project) -> Result<()> {
    let Some(revision) = project
        .revisions
        .iter()
        .find(|revision| Some(&revision.id) == project.current_revision.as_ref())
    else {
        return Ok(());
    };
    let mut names = HashSet::new();
    for name in [&revision.source, &revision.preview, &revision.program_base]
        .into_iter()
        .flatten()
    {
        names.insert(name.as_str());
    }
    if let Some(program) = &revision.program {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(program) {
            if value
                .get("schemaVersion")
                .and_then(serde_json::Value::as_u64)
                == Some(2)
            {
                let document: crate::cad_ir::Document = serde_json::from_value(value)?;
                for (_, hash) in crate::cad_ir::assets::references(&document)
                    .map_err(crate::cad_ir::app_error)?
                {
                    let file = project
                        .files
                        .iter()
                        .find(|file| {
                            file.sha256.as_deref() == Some(hash)
                                && std::path::Path::new(&file.name).extension().is_some_and(
                                    |extension| {
                                        extension.eq_ignore_ascii_case("step")
                                            || extension.eq_ignore_ascii_case("stp")
                                    },
                                )
                        })
                        .ok_or_else(|| AppError::Invalid("ASSET_MISSING".into()))?;
                    names.insert(file.name.as_str());
                }
            }
        }
    }
    for name in names {
        let file = project
            .files
            .iter()
            .find(|file| file.name == name)
            .ok_or_else(|| AppError::Invalid("Revision source file is missing".into()))?;
        crate::artifacts::read(state, &project.id, file)?;
    }
    Ok(())
}
