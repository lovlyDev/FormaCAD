//! Bridge from plugin plans to the project: model changes go only through the
//! existing Command API and model-apply transaction (validation, BREP build,
//! one new revision, nothing written on failure).
use super::{
    error::PluginError,
    service::{CallInput, Plan},
};
use crate::{
    cad_ir::Document,
    core::{AppError, AppState, Result},
    models::Project,
};

/// What a plugin may see of the open project.
pub async fn input(
    state: &AppState,
    project_id: Option<&str>,
    locale: &str,
    selection: Option<serde_json::Value>,
) -> Result<(CallInput, Option<Project>)> {
    let locale = if locale == "en" { "en" } else { "ru" }.to_owned();
    let selection = selection
        .map(|value| serde_json::to_string(&value))
        .transpose()?
        .filter(|text| text.len() <= 64 * 1024);
    let Some(project_id) = project_id else {
        return Ok((CallInput { locale, selection, ..Default::default() }, None));
    };
    let project = crate::projects::get(state, project_id).await?;
    let document = typed_document(&project);
    Ok((
        CallInput {
            locale,
            revision_id: project.current_revision.clone(),
            document: document.map(|(_, raw)| raw),
            selection,
        },
        Some(project),
    ))
}

fn typed_document(project: &Project) -> Option<(Document, String)> {
    let current = project.current_revision.as_ref()?;
    let raw = project
        .revisions
        .iter()
        .find(|revision| &revision.id == current)?
        .program
        .clone()?;
    let document = serde_json::from_str::<Document>(&raw).ok()?;
    Some((document, raw))
}

/// Apply a plan as one transaction. `expected_revision` guards against the
/// model changing between the plugin call and the commit.
pub async fn apply(
    state: &AppState,
    project: &Project,
    plan: Plan,
    expected_revision: Option<String>,
    prompt: String,
) -> Result<Project> {
    if plan.commands.is_empty() {
        return Ok(project.clone());
    }
    if project.current_revision != expected_revision {
        return Err(AppError::Invalid("Project changed; retry the CAD edit".into()));
    }
    let request = match (&expected_revision, typed_document(project)) {
        (Some(revision), Some(_)) => {
            crate::modeling::commands::prepare(state, project.id.clone(), plan.commands, prompt, revision.clone()).await?
        }
        (None, _) => {
            // Empty project: the plan builds the first CAD IR v2 document.
            let base = Document {
                schema_version: crate::cad_ir::SCHEMA_VERSION,
                revision_id: "plugin-base".into(),
                parameters: vec![],
                features: vec![],
                bodies: vec![],
            };
            let next = uuid::Uuid::new_v4().to_string();
            let document = crate::cad_ir::apply_commands(&base, "plugin-base", &next, &plan.commands)
                .map_err(crate::cad_ir::app_error)?;
            crate::modeling::apply::ApplyRequest::new(project.id.clone(), serde_json::to_string(&document)?, prompt, None)
        }
        (Some(_), None) => {
            return Err(PluginError::host(
                "PLUGIN_NEEDS_CAD_IR",
                "The current revision is not a CAD IR v2 model (for example an imported mesh)",
            )
            .for_plugin(&plan.plugin_id)
            .into())
        }
    };
    Ok(crate::modeling::apply::apply(state, request).await?.project)
}
