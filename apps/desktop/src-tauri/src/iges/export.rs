//! Export a captured committed STEP or an explicitly selected authored body.
use super::{failure, schema::Direction, task, ExportRequest};
use crate::{
    core::{AppState, Result},
    native::iges::client,
    project_repository::legacy,
};
use std::path::Path;
pub struct Prepared {
    pub bytes: Vec<u8>,
    pub report: super::schema::ExchangeReport,
    pub filename: String,
    pub revision_id: String,
}
pub async fn prepare_with_executable(
    state: &AppState,
    request: &ExportRequest,
    executable: &Path,
) -> Result<Prepared> {
    let snapshot = legacy::capture_project(state, &request.project_id).await?;
    if snapshot.project.current_revision.as_deref() != Some(request.expected_revision.as_str()) {
        return Err(failure("IGES_STALE", "Export revision changed"));
    }
    let revision = snapshot
        .project
        .revisions
        .iter()
        .find(|r| r.id == request.expected_revision)
        .ok_or_else(|| failure("IGES_EXACT_SOURCE_REQUIRED", "Revision missing"))?;
    let stored = if request.body_id.is_none() {
        let name = revision
            .source
            .as_ref()
            .filter(|name| {
                name.to_ascii_lowercase().ends_with(".step")
                    || name.to_ascii_lowercase().ends_with(".stp")
            })
            .ok_or_else(|| failure("IGES_EXACT_SOURCE_REQUIRED", "Stored STEP required"))?;
        Some(
            snapshot
                .project
                .files
                .iter()
                .find(|f| &f.name == name)
                .ok_or_else(|| failure("IGES_EXACT_SOURCE_REQUIRED", "Stored STEP missing"))?
                .clone(),
        )
    } else {
        None
    };
    let document = if let Some(body_id) = &request.body_id {
        let source = revision
            .program
            .as_ref()
            .ok_or_else(|| failure("IGES_EXACT_SOURCE_REQUIRED", "Authored body required"))?;
        let doc: crate::cad_ir::Document = serde_json::from_str(source)?;
        doc.validate().map_err(crate::cad_ir::app_error)?;
        if !doc.bodies.iter().any(|body| &body.id == body_id) {
            return Err(failure("IGES_INVALID_SCOPE", "Selected body missing"));
        }
        Some((source.clone(), doc))
    } else {
        None
    };
    crate::permissions::consume(state, &request.project_id, "export_file").await?;
    task::run(
        state,
        &request.project_id,
        "iges_export",
        |cancel| async move {
            legacy::recheck(state, &snapshot).await?;
            let workspace = crate::project_access::transient::Workspace::create(&state.root)?;
            let source = if let Some((program, document)) = &document {
                crate::native::assets::stage_project_assets(
                    state,
                    &snapshot.project,
                    document,
                    &workspace.path,
                )?;
                let selected = request
                    .body_id
                    .as_deref()
                    .ok_or_else(|| failure("IGES_INVALID_SCOPE", "Explicit body required"))?;
                if !crate::native::worker::build_selected_with_executable(
                    program,
                    selected,
                    &workspace.path,
                    cancel.clone(),
                    executable,
                )
                .await?
                {
                    return Err(failure("IGES_EXACT_SOURCE_REQUIRED", "CAD IR v2 required"));
                }
                let path = crate::security::guarded(&workspace.path, Path::new("model.step"))?;
                if std::fs::metadata(&path)?.len() > crate::artifacts::MAX_FILE_BYTES as u64 {
                    return Err(failure("IGES_SIZE_INVALID", "Selected body exceeds limit"));
                }
                std::fs::read(path)?
            } else {
                crate::artifacts::read(
                    state,
                    &snapshot.project.id,
                    stored.as_ref().ok_or_else(|| {
                        failure("IGES_EXACT_SOURCE_REQUIRED", "STEP source missing")
                    })?,
                )?
            };
            let output = client::run(
                &workspace.path,
                &source,
                Direction::Export,
                cancel.clone(),
                executable,
            )
            .await?;
            task::check(&cancel)?;
            legacy::recheck(state, &snapshot).await?;
            if let Some(file) = &stored {
                if crate::artifacts::read(state, &snapshot.project.id, file)? != source {
                    return Err(failure("IGES_STALE", "Stored STEP changed"));
                }
            }
            if let Some((_, document)) = &document {
                crate::native::assets::stage_project_assets(
                    state,
                    &snapshot.project,
                    document,
                    &workspace.path,
                )?;
            }
            task::check(&cancel)?;
            let name = snapshot
                .project
                .name
                .chars()
                .map(|c| {
                    if c.is_alphanumeric() || c == '-' || c == '_' {
                        c
                    } else {
                        '-'
                    }
                })
                .collect::<String>();
            Ok(Prepared {
                bytes: output.geometry,
                report: output.report,
                filename: format!("{name}-r{}.iges", snapshot.project.revisions.len()),
                revision_id: request.expected_revision.clone(),
            })
        },
    )
    .await
}
