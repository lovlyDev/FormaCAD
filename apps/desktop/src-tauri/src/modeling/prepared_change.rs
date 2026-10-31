//! Pure saved-model preparation, never a storage commit or writer capability.
//! Output bytes are synthetic in unit tests; geometry validity belongs to the worker.
use super::build::GeneratedModel;
use crate::{
    core::{AppError, Result},
    models::{Project, ProjectFile, Revision},
};
use std::sync::atomic::{AtomicBool, Ordering};

/// A normalized candidate. Only modeling's preparer can construct it.
/// The backend must still authorize, compare the original source and commit history.
pub(crate) struct PreparedModelChange {
    project: Project,
    pending: Vec<(String, Vec<u8>)>,
}

impl PreparedModelChange {
    pub(crate) fn into_parts(self) -> (Project, Vec<(String, Vec<u8>)>) {
        (self.project, self.pending)
    }
}

/// Preserve the managed assembly/normalization/cancellation order. The host
/// supplies one timestamp; this function performs no clock, filesystem or SQL IO.
pub(super) fn prepare(
    mut project: Project,
    model: GeneratedModel,
    job: String,
    program: String,
    prompt: String,
    now: String,
    cancel: &AtomicBool,
) -> Result<PreparedModelChange> {
    let step_name = format!("model-{job}.step");
    let preview_name = format!("preview-{job}.glb");
    for (name, bytes) in [(&step_name, &model.step), (&preview_name, &model.preview)] {
        project.files.push(ProjectFile {
            name: name.clone(),
            size: bytes.len() as u64,
            kind: "model".into(),
            data: Some(crate::artifacts::data_url(name, bytes)),
            sha256: None,
        });
    }
    project.revisions.push(Revision {
        program: Some(program),
        program_base: model.program_base,
        id: job.clone(),
        parent: project.current_revision.clone(),
        created_at: now.clone(),
        prompt,
        source: Some(step_name),
        preview: Some(preview_name),
        parameters: crate::models::Parameters {
            kind: "blank".into(),
            width: 120.0,
            depth: 65.0,
            height: 60.0,
            thickness: 5.0,
            hole_diameter: 8.0,
            holes: 4,
        },
    });
    project.current_revision = Some(job);
    project.updated_at = now;
    let pending = crate::artifacts::normalize(&mut project)?;
    if cancel.load(Ordering::Relaxed) {
        return Err(AppError::Invalid("CAD_TASK_CANCELLED".into()));
    }
    project.validate()?;
    Ok(PreparedModelChange { project, pending })
}

#[cfg(test)]
mod tests;
