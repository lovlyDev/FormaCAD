use super::build::GeneratedModel;
use crate::{
    core::{AppError, AppState, Result},
    models::{Project, ProjectFile, Revision},
};
use std::sync::atomic::{AtomicBool, Ordering};

pub(super) fn program_for_revision(
    program: &str,
    typed: bool,
    revision_id: &str,
) -> Result<String> {
    if typed {
        let mut value: serde_json::Value = serde_json::from_str(program)?;
        if value.get("schemaVersion").is_some() {
            value["revisionId"] = revision_id.into();
            return Ok(serde_json::to_string(&value)?);
        }
    }
    Ok(program.trim().to_string())
}

/// Called only under the host write lock after expected-head validation.
pub(super) async fn commit(
    state: &AppState,
    mut project: Project,
    model: GeneratedModel,
    job: String,
    program: String,
    prompt: String,
    cancel: &AtomicBool,
) -> Result<Project> {
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
    let now = chrono::Utc::now().to_rfc3339();
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
    crate::projects::persist(state, project, pending, None).await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ir_revision_is_bound_to_saved_project_revision() {
        let saved = program_for_revision(
            r#"{"schemaVersion":2,"revisionId":"draft"}"#,
            true,
            "committed",
        )
        .unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&saved).unwrap()["revisionId"],
            "committed"
        );
        let old = r#" {"version":1,"features":[],"output":"part"} "#;
        assert_eq!(
            program_for_revision(old, true, "committed").unwrap(),
            old.trim()
        );
    }
}
