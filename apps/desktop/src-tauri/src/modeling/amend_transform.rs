//! F9 host preparation. Geometry still goes through the normal apply/permission/queue boundary.
pub mod ipc;
use super::apply::ApplyRequest;
use crate::{
    cad_ir::{Document, Operation},
    core::{AppError, AppState, Result},
    models::Project,
    project_history::AmendCapture,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LastTransformCommand {
    pub command_index: usize,
    pub body_id: String,
    pub feature_id: String,
    pub name: String,
    pub matrix: [f64; 12],
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LastTransformStatus {
    pub expected_revision: String,
    pub expected_generation: u64,
    pub transaction_id: String,
    pub base_revision: String,
    pub commands: Vec<LastTransformCommand>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TransformReplacement {
    pub command_index: usize,
    pub matrix: [f64; 12],
}
fn stale() -> AppError {
    AppError::Invalid("AMEND_STALE".into())
}

fn read_document(project: &Project, id: &str) -> Option<Document> {
    let revision = project
        .revisions
        .iter()
        .find(|revision| revision.id == id)?;
    let document: Document = serde_json::from_str(revision.program.as_deref()?).ok()?;
    if document.revision_id != id || document.validate().is_err() {
        return None;
    }
    Some(document)
}

/// Conservative support: the last transaction must consist solely of independent body transforms.
fn describe(before: &Document, current: &Document) -> Option<Vec<LastTransformCommand>> {
    if current.parameters != before.parameters
        || !current.features.starts_with(&before.features)
        || current.bodies.len() != before.bodies.len()
        || current
            .bodies
            .iter()
            .zip(&before.bodies)
            .any(|(a, b)| a.id != b.id || a.name != b.name)
    {
        return None;
    }
    let added = &current.features[before.features.len()..];
    if added.is_empty() || added.len() > 256 {
        return None;
    }
    let mut result = Vec::new();
    let mut touched = HashSet::new();
    for (command_index, feature) in added.iter().enumerate() {
        let Operation::Transform {
            body_feature_id,
            matrix,
        } = &feature.operation
        else {
            return None;
        };
        if feature.suppressed {
            return None;
        }
        let index = current
            .bodies
            .iter()
            .position(|body| body.source_feature_id == feature.id)?;
        let body = &current.bodies[index];
        if before.bodies[index].source_feature_id != *body_feature_id
            || !touched.insert(body.id.clone())
        {
            return None;
        }
        result.push(LastTransformCommand {
            command_index,
            body_id: body.id.clone(),
            feature_id: feature.id.clone(),
            name: feature.name.clone(),
            matrix: *matrix,
        });
    }
    if current
        .bodies
        .iter()
        .zip(&before.bodies)
        .any(|(a, b)| a.source_feature_id != b.source_feature_id && !touched.contains(&a.id))
    {
        return None;
    }
    Some(result)
}
fn definition(
    project: &Project,
    capture: &AmendCapture,
) -> Option<(Document, Document, Vec<LastTransformCommand>)> {
    let before = read_document(project, &capture.base_revision)?;
    let current = read_document(project, &capture.expected_revision)?;
    let commands = describe(&before, &current)?;
    Some((before, current, commands))
}
pub async fn status(state: &AppState, project_id: &str) -> Result<Option<LastTransformStatus>> {
    crate::security::valid_id(project_id)?;
    let project = crate::projects::get(state, project_id).await?;
    let history = crate::project_history::load_state(&state.pool, &project).await?;
    let Some(capture) = history.capture_amend(&project)? else {
        return Ok(None);
    };
    let Some((_, _, commands)) = definition(&project, &capture) else {
        return Ok(None);
    };
    Ok(Some(LastTransformStatus {
        expected_revision: capture.expected_revision,
        expected_generation: capture.generation,
        transaction_id: capture.transaction_id,
        base_revision: capture.base_revision,
        commands,
    }))
}

fn replace(
    current: &Document,
    commands: &[LastTransformCommand],
    replacements: &[TransformReplacement],
) -> Result<Document> {
    if replacements.is_empty() || replacements.len() > commands.len() {
        return Err(stale());
    }
    let mut next = current.clone();
    let mut seen = HashSet::new();
    for replacement in replacements {
        if !seen.insert(replacement.command_index) {
            return Err(stale());
        }
        let command = commands.get(replacement.command_index).ok_or_else(stale)?;
        let feature = next
            .features
            .iter_mut()
            .find(|feature| feature.id == command.feature_id)
            .ok_or_else(stale)?;
        let Operation::Transform { matrix, .. } = &mut feature.operation else {
            return Err(stale());
        };
        *matrix = replacement.matrix;
    }
    next.validate().map_err(crate::cad_ir::app_error)?;
    if next == *current {
        return Err(AppError::Invalid("NO_CHANGE".into()));
    }
    Ok(next)
}
pub async fn prepare(
    state: &AppState,
    project_id: String,
    expected_revision: String,
    expected_generation: u64,
    transaction_id: String,
    replacements: Vec<TransformReplacement>,
) -> Result<ApplyRequest> {
    crate::security::valid_id(&project_id)?;
    crate::security::valid_id(&expected_revision)?;
    let project = crate::projects::get(state, &project_id).await?;
    let history = crate::project_history::load_state(&state.pool, &project).await?;
    let capture = history.capture_amend(&project)?.ok_or_else(stale)?;
    if capture.expected_revision != expected_revision
        || capture.generation != expected_generation
        || capture.transaction_id != transaction_id
    {
        return Err(stale());
    }
    let (_, current, commands) = definition(&project, &capture).ok_or_else(stale)?;
    let mut next = replace(&current, &commands, &replacements)?;
    let revision = uuid::Uuid::new_v4().to_string();
    next.revision_id = revision.clone();
    let mut request = ApplyRequest::new(
        project_id,
        serde_json::to_string(&next)?,
        format!("history:amend:{expected_revision}"),
        Some(expected_revision),
    );
    request.next_revision = Some(revision);
    request.history_amend = Some(capture);
    Ok(request)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cad_ir::{apply_commands, Command};
    fn before() -> Document {
        serde_json::from_value(serde_json::json!({"schemaVersion":2,"revisionId":"base","parameters":[],"features":[
            {"id":"profile","name":"Profile","operation":{"type":"rectangle","width":{"kind":"literal","mm":20},"depth":{"kind":"literal","mm":10}}},
            {"id":"pad","name":"Pad","operation":{"type":"extrude","sketchId":"profile","distance":{"kind":"literal","mm":5}}}
        ],"bodies":[{"id":"body","name":"Body","sourceFeatureId":"pad"}]})).unwrap()
    }
    fn moved() -> (Document, Document) {
        let before = before();
        let current = apply_commands(
            &before,
            "base",
            "moved",
            &[Command::TransformBody {
                body_id: "body".into(),
                feature_id: "move".into(),
                name: "Move".into(),
                matrix: [1., 0., 0., 10., 0., 1., 0., 0., 0., 0., 1., 0.],
            }],
        )
        .unwrap();
        (before, current)
    }
    #[test]
    fn replacement_edits_original_transform_not_a_stacked_modifier() {
        let (before, current) = moved();
        let commands = describe(&before, &current).unwrap();
        let mut matrix = commands[0].matrix;
        matrix[3] = 20.;
        let next = replace(
            &current,
            &commands,
            &[TransformReplacement {
                command_index: 0,
                matrix,
            }],
        )
        .unwrap();
        assert_eq!(next.features.len(), current.features.len());
        assert_eq!(next.features.last().unwrap().id, "move");
        assert!(
            matches!(&next.features.last().unwrap().operation, Operation::Transform { body_feature_id, matrix } if body_feature_id == "pad" && matrix[3] == 20.)
        );
        matrix[3] = 0.;
        assert!(replace(
            &current,
            &commands,
            &[TransformReplacement {
                command_index: 0,
                matrix
            }]
        )
        .is_ok());
    }
    #[test]
    fn altered_prefix_mixed_commands_and_invalid_replacements_are_rejected() {
        let (before, current) = moved();
        let mut mixed = current.clone();
        mixed.parameters.push(crate::cad_ir::Parameter {
            id: "new".into(),
            name: "New".into(),
            value_mm: 1.,
        });
        assert!(describe(&before, &mixed).is_none());
        mixed = current.clone();
        mixed.features[0].name = "Changed".into();
        assert!(describe(&before, &mixed).is_none());
        let commands = describe(&before, &current).unwrap();
        assert!(replace(&current, &commands, &[]).is_err());
        let mut matrix = commands[0].matrix;
        matrix[0] = 0.;
        assert!(replace(
            &current,
            &commands,
            &[TransformReplacement {
                command_index: 0,
                matrix
            }]
        )
        .is_err());
        assert!(replace(
            &current,
            &commands,
            &[TransformReplacement {
                command_index: 1,
                matrix: commands[0].matrix
            }]
        )
        .is_err());
    }
}
