use super::{Body, Command, Document, ErrorCode, Result, ValidationError};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BodyGroup {
    pub id: String,
    pub name: String,
    pub body_ids: Vec<String>,
}

pub(super) fn validate(groups: &[BodyGroup], bodies: &[Body]) -> Result<()> {
    if groups.len() > 256
        || groups
            .iter()
            .map(|group| group.body_ids.len())
            .sum::<usize>()
            > 16384
    {
        return Err(ValidationError::new(ErrorCode::InvalidDocument, None));
    }
    let body_ids: HashSet<_> = bodies.iter().map(|body| body.id.as_str()).collect();
    for group in groups {
        let mut unique = HashSet::new();
        for id in &group.body_ids {
            if !body_ids.contains(id.as_str()) {
                return Err(ValidationError::new(
                    ErrorCode::MissingTarget,
                    Some(id.clone()),
                ));
            }
            if !unique.insert(id) {
                return Err(ValidationError::new(
                    ErrorCode::DuplicateId,
                    Some(id.clone()),
                ));
            }
        }
    }
    Ok(())
}

pub(super) fn apply(document: &mut Document, command: &Command) -> Result<()> {
    let (group_id, members, add) = match command {
        Command::GroupCreate { group } => {
            document.groups.push(group.clone());
            return Ok(());
        }
        Command::GroupAdd { group_id, body_ids } => (group_id, Some(body_ids), true),
        Command::GroupRemove { group_id, body_ids } => (group_id, body_ids.as_ref(), false),
        _ => return Err(ValidationError::new(ErrorCode::UnsupportedField, None)),
    };
    let index = document
        .groups
        .iter()
        .position(|group| group.id == *group_id)
        .ok_or_else(|| ValidationError::new(ErrorCode::MissingTarget, Some(group_id.clone())))?;
    let Some(members) = members else {
        document.groups.remove(index);
        return Ok(());
    };
    let body_ids: HashSet<_> = document
        .bodies
        .iter()
        .map(|body| body.id.as_str())
        .collect();
    if let Some(id) = members.iter().find(|id| !body_ids.contains(id.as_str())) {
        return Err(ValidationError::new(
            ErrorCode::MissingTarget,
            Some(id.clone()),
        ));
    }
    let group = &mut document.groups[index];
    let mut unique: HashSet<_> = group.body_ids.iter().cloned().collect();
    if add {
        group.body_ids.extend(
            members
                .iter()
                .filter(|id| unique.insert((*id).clone()))
                .cloned(),
        );
    } else {
        let removed: HashSet<_> = members.iter().collect();
        group.body_ids.retain(|id| !removed.contains(id));
    }
    Ok(())
}
