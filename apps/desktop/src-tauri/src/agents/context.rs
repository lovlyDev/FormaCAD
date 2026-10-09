//! Bounded, explicit project context sent to the CAD planner.
use super::selection::SelectionContext;
use crate::models::Project;

pub fn project_context(
    project: &Project,
    prompt: &str,
    image_names: &[String],
    selection: Option<&SelectionContext>,
) -> serde_json::Value {
    let current = project
        .revisions
        .iter()
        .find(|revision| Some(&revision.id) == project.current_revision.as_ref());
    let history: Vec<_> = project
        .messages
        .iter()
        .rev()
        .filter(|message| message.role == "user" || message.role == "assistant")
        .take(6)
        .map(|message| {
            serde_json::json!({
                "role": message.role,
                "text": message.text.chars().take(2000).collect::<String>()
            })
        })
        .collect();
    serde_json::json!({
        "name": project.name,
        "dimensionsUnit": "mm",
        "cadKernelReady": true,
        "referenceImages": image_names,
        "currentProgram": current.and_then(|revision| revision.program.as_ref()),
        "legacyModel": current.filter(|revision| revision.source.is_none()).map(|revision| &revision.parameters),
        "baseStepAvailable": current.and_then(|revision| revision.source.as_ref())
            .is_some_and(|source| source.ends_with(".step") || source.ends_with(".stp")),
        "recentMessagesNewestFirst": history,
        "selection": selection,
        "request": prompt
    })
}
