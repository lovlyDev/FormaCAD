//! Best-effort mirrors after the durable SQL transaction. Guards remain mandatory.
use crate::{core::Result, models::Project, security};
use std::path::Path;

pub(super) fn update(root: &Path, project: &Project, raw: &str) {
    if let Err(error) = project_json(root, &project.id, raw) {
        tracing::warn!(%error, "Project JSON mirror failed; SQLite copy is committed");
    }
    if let Some(current) = project
        .revisions
        .iter()
        .find(|r| Some(&r.id) == project.current_revision.as_ref())
    {
        if let Some(program) = &current.program {
            if let Err(error) = write(root, &project.id, "workspace/model.py", program.as_bytes()) {
                tracing::warn!(%error, "Could not mirror current CAD source; SQLite copy is committed");
            }
        }
        if let Err(error) = parameters(root, &project.id, &current.parameters) {
            tracing::warn!(%error, "Could not mirror current parameters; SQLite copy is committed");
        }
    }
}

fn project_json(root: &Path, project_id: &str, raw: &str) -> Result<()> {
    let target = security::guarded(root, &Path::new(project_id).join("project.json"))?;
    let temporary = security::guarded(
        root,
        &Path::new(project_id).join("metadata/project.json.tmp"),
    )?;
    std::fs::write(&temporary, raw)?;
    std::fs::rename(temporary, target)?;
    Ok(())
}
fn write(root: &Path, project_id: &str, relative: &str, bytes: &[u8]) -> Result<()> {
    let target = security::guarded(root, &Path::new(project_id).join(relative))?;
    std::fs::write(target, bytes)?;
    Ok(())
}
fn parameters(root: &Path, project_id: &str, parameters: &crate::models::Parameters) -> Result<()> {
    write(
        root,
        project_id,
        "workspace/model.parameters.json",
        &serde_json::to_vec_pretty(parameters)?,
    )
}
