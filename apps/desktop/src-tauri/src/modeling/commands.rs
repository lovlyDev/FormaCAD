use super::apply::ApplyRequest;
use crate::{
    cad_ir::{Command, Document},
    core::{AppError, AppState, Result},
};

pub async fn prepare(
    state: &AppState,
    project_id: String,
    commands: Vec<Command>,
    prompt: String,
    expected_revision: String,
) -> Result<ApplyRequest> {
    if commands.len() > 256 {
        return Err(AppError::Invalid("Too many CAD commands".into()));
    }
    let project = crate::projects::get(state, &project_id).await?;
    if project.current_revision.as_deref() != Some(&expected_revision) {
        return Err(AppError::Invalid(
            "Project changed; retry the CAD edit".into(),
        ));
    }
    let source = project
        .revisions
        .iter()
        .find(|revision| revision.id == expected_revision)
        .and_then(|revision| revision.program.as_deref())
        .ok_or_else(|| AppError::Invalid("Current revision has no CAD IR".into()))?;
    let current: Document = serde_json::from_str(source)?;
    if current.revision_id != expected_revision {
        return Err(AppError::Invalid(
            "CAD IR revision does not match project".into(),
        ));
    }
    let next = uuid::Uuid::new_v4().to_string();
    let updated = crate::cad_ir::apply_commands(&current, &expected_revision, &next, &commands)
        .map_err(crate::cad_ir::app_error)?;
    let mut request = ApplyRequest::new(
        project_id,
        serde_json::to_string(&updated)?,
        prompt,
        Some(expected_revision),
    );
    request.next_revision = Some(next);
    Ok(request)
}
