use super::build::GeneratedModel;
use crate::{
    core::{AppState, Result},
    models::Project,
};
use std::sync::atomic::AtomicBool;

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
    project: Project,
    model: GeneratedModel,
    job: String,
    program: String,
    prompt: String,
    cancel: &AtomicBool,
) -> Result<Project> {
    let now = chrono::Utc::now().to_rfc3339();
    let prepared =
        super::prepared_change::prepare(project, model, job, program, prompt, now, cancel)?;
    let (project, pending) = prepared.into_parts();
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
