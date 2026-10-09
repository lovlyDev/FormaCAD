//! Deliberate conversation redaction; geometry and executable model sources
//! retain their original IDs, labels and content and are not anonymized.
use crate::{core::Result, models::Project};
use std::collections::HashSet;

pub(super) fn conversation_free(source: &Project) -> Result<Project> {
    source.validate()?;
    let mut project = source.clone();
    project.messages.clear();
    project.exports.clear();
    project.thumbnail = None;
    project.thumbnail_revision = None;
    for revision in &mut project.revisions {
        revision.prompt.clear();
    }
    // Arbitrary legacy scripts may open extra attachments by filename. Dropping
    // them would silently break their rebuild: retain them and document this limit.
    let scripted = project.revisions.iter().any(|revision| {
        revision
            .program
            .as_ref()
            .is_some_and(|program| serde_json::from_str::<serde_json::Value>(program).is_err())
    });
    if !scripted {
        let mut asset_hashes = HashSet::new();
        for revision in &project.revisions {
            if let Some(program) = &revision.program {
                let value: serde_json::Value = serde_json::from_str(program)?;
                if value
                    .get("schemaVersion")
                    .and_then(serde_json::Value::as_u64)
                    == Some(2)
                {
                    let document: crate::cad_ir::Document = serde_json::from_value(value)?;
                    for (_, hash) in crate::cad_ir::assets::references(&document)
                        .map_err(crate::cad_ir::app_error)?
                    {
                        asset_hashes.insert(hash.to_owned());
                    }
                }
            }
        }
        let required: HashSet<&str> = project
            .revisions
            .iter()
            .flat_map(|revision| [&revision.source, &revision.preview, &revision.program_base])
            .flatten()
            .map(String::as_str)
            .collect();
        project.files.retain(|file| {
            required.contains(file.name.as_str())
                || (file
                    .sha256
                    .as_ref()
                    .is_some_and(|hash| asset_hashes.contains(hash))
                    && std::path::Path::new(&file.name)
                        .extension()
                        .is_some_and(|extension| {
                            extension.eq_ignore_ascii_case("step")
                                || extension.eq_ignore_ascii_case("stp")
                        }))
        });
    }
    project.validate()?;
    Ok(project)
}
