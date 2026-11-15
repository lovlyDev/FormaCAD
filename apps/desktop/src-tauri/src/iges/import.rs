use super::{
    failure, task, CommitRequest, ImportOutcome as Imported, ImportPreview as Preview,
    ImportRequest,
};
use crate::{
    core::{AppState, Result},
    models::{Project, ProjectFile},
    native::iges::client,
    project_repository::legacy,
};
use std::path::Path;
fn input(request: &ImportRequest) -> Result<Vec<u8>> {
    crate::security::valid_id(&request.project_id)?;
    crate::security::file_name(&request.name)
        .map_err(|e| failure("IGES_INVALID_INPUT", e.to_string()))?;
    let ext = Path::new(&request.name)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !["igs", "iges"].contains(&ext.as_str()) || request.name.chars().count() > 120 {
        return Err(failure(
            "IGES_INVALID_INPUT",
            "Select an IGES filename up to 120 characters",
        ));
    }
    crate::artifacts::decode_data(&request.data)
        .map_err(|e| failure("IGES_INVALID_INPUT", e.to_string()))
}
fn validate(project: &Project, request: &ImportRequest, bytes: &[u8]) -> Result<()> {
    if project.current_revision != request.expected_revision {
        return Err(failure("IGES_STALE", "Project revision changed"));
    }
    if project.files.iter().any(|f| {
        f.name == request.name
            && f.sha256.as_deref() != Some(crate::artifacts::digest(bytes).as_str())
    }) {
        return Err(failure(
            "IGES_NAME_CONFLICT",
            "A different attachment already has this filename",
        ));
    }
    Ok(())
}
pub async fn preview_with_executable(
    state: &AppState,
    request: &ImportRequest,
    executable: &Path,
) -> Result<Preview> {
    let bytes = input(request)?;
    let snapshot = legacy::capture_project(state, &request.project_id).await?;
    validate(&snapshot.project, request, &bytes)?;
    task::run(
        state,
        &request.project_id,
        "iges_preview",
        |cancel| async move {
            legacy::recheck(state, &snapshot).await?;
            let workspace = crate::project_access::transient::Workspace::create(&state.root)?;
            let output = client::run(
                &workspace.path,
                &bytes,
                super::schema::Direction::Import,
                cancel.clone(),
                executable,
            )
            .await?;
            legacy::recheck(state, &snapshot).await?;
            task::check(&cancel)?;
            Ok(Preview {
                report: output.report,
                preview_data: crate::artifacts::data_url(
                    "preview.glb",
                    &output
                        .preview
                        .ok_or_else(|| failure("IGES_REPORT_INVALID", "Preview missing"))?,
                ),
            })
        },
    )
    .await
}
pub async fn commit_with_executable(
    state: &AppState,
    request: &CommitRequest,
    executable: &Path,
) -> Result<Imported> {
    let _access = crate::project_access::ensure_write(state, &request.input.project_id)?;
    let bytes = input(&request.input)?;
    if crate::artifacts::digest(&bytes) != request.source_sha256
        || request.prompt.trim().is_empty()
        || request.prompt.len() > 16000
    {
        return Err(failure("IGES_STALE", "Confirmed input changed"));
    }
    let snapshot = legacy::capture_project(state, &request.input.project_id).await?;
    validate(&snapshot.project, &request.input, &bytes)?;
    crate::permissions::consume(state, &request.input.project_id, "convert_file").await?;
    task::run(
        state,
        &request.input.project_id,
        "iges_import",
        |cancel| async move {
            legacy::recheck(state, &snapshot).await?;
            let workspace = crate::project_access::transient::Workspace::create(&state.root)?;
            let output = client::run(
                &workspace.path,
                &bytes,
                super::schema::Direction::Import,
                cancel.clone(),
                executable,
            )
            .await?;
            task::check(&cancel)?;
            let _writes = state.writes.lock().await;
            legacy::recheck(state, &snapshot).await?;
            task::check(&cancel)?;
            let mut project = snapshot.project.clone();
            if !project.files.iter().any(|f| f.name == request.input.name) {
                project.files.push(ProjectFile {
                    name: request.input.name.clone(),
                    size: bytes.len() as u64,
                    kind: "model".into(),
                    data: Some(crate::artifacts::data_url(&request.input.name, &bytes)),
                    sha256: None,
                });
            }
            let job = uuid::Uuid::new_v4().to_string();
            let program = super::document::source(&job, &request.input.name, &output.geometry)?;
            let project = crate::modeling::commit::commit(
                state,
                project,
                crate::modeling::build::GeneratedModel {
                    geometry: Some((
                        output.geometry,
                        output
                            .preview
                            .ok_or_else(|| failure("IGES_REPORT_INVALID", "Preview missing"))?,
                    )),
                    program_base: None,
                },
                crate::modeling::commit::CommitMetadata {
                    job,
                    program,
                    prompt: request.prompt.clone(),
                    amend: None,
                },
                &cancel,
            )
            .await?;
            Ok(Imported {
                project,
                report: output.report,
            })
        },
    )
    .await
}
