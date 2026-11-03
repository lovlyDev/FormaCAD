use super::prepared_change::PreparedModelChange;
use crate::{
    core::{AppState, Result},
    models::Project,
};

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

pub(super) struct AmendContext<'a> {
    pub before: &'a Project,
    pub history: &'a crate::project_history::HistoryState,
    pub capture: &'a crate::project_history::AmendCapture,
}

/// Persist only a candidate assembled by the common orchestrator under its host locks.
pub(super) async fn persist_prepared(
    state: &AppState,
    prepared: PreparedModelChange,
    amend: Option<AmendContext<'_>>,
) -> Result<Project> {
    let (project, pending) = prepared.into_parts();
    match amend {
        Some(amend) => {
            let intent = crate::project_history::prepare_amend(
                amend.before,
                &project,
                amend.history,
                amend.capture,
            )?;
            crate::projects::persist_with_history(state, project, pending, None, intent).await
        }
        None => crate::projects::persist(state, project, pending, None).await,
    }
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
