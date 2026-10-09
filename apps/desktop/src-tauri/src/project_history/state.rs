use crate::{
    core::{AppError, Result},
    models::{Project, Revision},
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HistoryState {
    entries: Vec<Option<String>>,
    cursor: usize,
    head: Option<String>,
    generation: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Undo,
    Redo,
}

pub struct PreparedHistory {
    pub(super) before: HistoryState,
    pub(super) after: HistoryState,
    pub(super) before_project: String,
    pub(super) after_project: String,
    pub(super) kind: &'static str,
    pub(super) target: Option<String>,
}

fn invalid(code: &str) -> AppError {
    AppError::Invalid(code.into())
}

/// History materialization changes identity, never the saved model snapshot.
pub(super) fn same_snapshot(baseline: &Revision, current: &Revision) -> bool {
    fn canonical(program: &Option<String>) -> Option<String> {
        program.as_ref().map(
            |program| match serde_json::from_str::<serde_json::Value>(program) {
                Ok(mut value) if value.get("schemaVersion").is_some() => {
                    if let Some(object) = value.as_object_mut() {
                        object.remove("revisionId");
                    }
                    value.to_string()
                }
                _ => program.clone(),
            },
        )
    }
    baseline.parameters == current.parameters
        && baseline.source == current.source
        && baseline.preview == current.preview
        && baseline.program_base == current.program_base
        && canonical(&baseline.program) == canonical(&current.program)
}

impl HistoryState {
    /// Older projects have no cursor row. Recover only the current parent chain;
    /// do not infer abandoned siblings as redo actions.
    pub fn from_project(project: &Project) -> Result<Self> {
        project.validate()?;
        let revisions: HashMap<&str, _> = project
            .revisions
            .iter()
            .map(|revision| (revision.id.as_str(), revision))
            .collect();
        let mut path = Vec::new();
        let mut next = project.current_revision.as_ref();
        while let Some(id) = next {
            let revision = revisions
                .get(id.as_str())
                .ok_or_else(|| invalid("HISTORY_INVALID_STATE"))?;
            path.push(Some(id.clone()));
            next = revision.parent.as_ref();
        }
        path.reverse();
        path.insert(0, None);
        Ok(Self {
            cursor: path.len() - 1,
            entries: path,
            head: project.current_revision.clone(),
            generation: 0,
        })
    }

    pub fn can_undo(&self) -> bool {
        self.cursor > 0
    }
    pub fn can_redo(&self) -> bool {
        self.cursor + 1 < self.entries.len()
    }

    pub(super) fn validate(&self, project: &Project) -> Result<()> {
        let ids: HashSet<&str> = project
            .revisions
            .iter()
            .map(|revision| revision.id.as_str())
            .collect();
        let mut logical_ids = HashSet::new();
        if self.entries.is_empty()
            || self.entries.len() > 10001
            || self.entries[0].is_some()
            || self.cursor >= self.entries.len()
            || self.head != project.current_revision
            || self.entries.iter().skip(1).any(|entry| {
                entry
                    .as_deref()
                    .is_none_or(|id| !ids.contains(id) || !logical_ids.insert(id))
            })
        {
            return Err(invalid("HISTORY_INVALID_STATE"));
        }
        match (&self.entries[self.cursor], &self.head) {
            (None, None) => (),
            (Some(logical), Some(head)) => {
                let baseline = project
                    .revisions
                    .iter()
                    .find(|revision| &revision.id == logical)
                    .ok_or_else(|| invalid("HISTORY_INVALID_STATE"))?;
                let current = project
                    .revisions
                    .iter()
                    .find(|revision| &revision.id == head)
                    .ok_or_else(|| invalid("HISTORY_INVALID_STATE"))?;
                if !same_snapshot(baseline, current) {
                    return Err(invalid("HISTORY_INVALID_STATE"));
                }
            }
            _ => return Err(invalid("HISTORY_INVALID_STATE")),
        }
        Ok(())
    }

    pub(super) fn edited(&self, project: &Project) -> Result<Self> {
        let mut state = self.clone();
        state.entries.truncate(state.cursor + 1);
        let revision = project
            .current_revision
            .clone()
            .ok_or_else(|| invalid("HISTORY_INVALID_STATE"))?;
        state.entries.push(Some(revision.clone()));
        state.cursor += 1;
        state.head = Some(revision);
        state.generation = state
            .generation
            .checked_add(1)
            .ok_or_else(|| invalid("HISTORY_INVALID_STATE"))?;
        state.validate(project)?;
        Ok(state)
    }
}

/// Prepare on a copy. No database, artifact, permission, or original mutation.
pub fn prepare(
    current: &Project,
    history: &HistoryState,
    direction: Direction,
    expected_revision: Option<&str>,
    next_id: &str,
    now: &str,
) -> Result<(Project, PreparedHistory)> {
    current.validate()?;
    history.validate(current)?;
    if current.current_revision.as_deref() != expected_revision {
        return Err(invalid("Project changed; retry the CAD edit"));
    }
    crate::security::valid_id(next_id)?;
    if current
        .revisions
        .iter()
        .any(|revision| revision.id == next_id)
    {
        return Err(invalid("Duplicate revision ID"));
    }
    let (cursor, kind) = match direction {
        Direction::Undo if history.can_undo() => (history.cursor - 1, "undo"),
        Direction::Redo if history.can_redo() => (history.cursor + 1, "redo"),
        Direction::Undo => return Err(invalid("HISTORY_NO_UNDO")),
        Direction::Redo => return Err(invalid("HISTORY_NO_REDO")),
    };
    let target = history.entries[cursor].clone();
    let mut project = current.clone();
    if let Some(target_id) = &target {
        let mut revision = current
            .revisions
            .iter()
            .find(|revision| &revision.id == target_id)
            .ok_or_else(|| invalid("HISTORY_INVALID_STATE"))?
            .clone();
        revision.id = next_id.to_owned();
        revision.parent = current.current_revision.clone();
        revision.created_at = now.to_owned();
        // Machine labels are translated at the UI boundary, not stored in a locale.
        revision.prompt = format!("history:{kind}:{target_id}");
        if let Some(program) = &revision.program {
            if let Ok(mut value) = serde_json::from_str::<serde_json::Value>(program) {
                if value.get("schemaVersion").is_some() {
                    value["revisionId"] = serde_json::Value::String(next_id.to_owned());
                    if value.get("schemaVersion").and_then(|value| value.as_u64()) == Some(2) {
                        let document: crate::cad_ir::Document =
                            serde_json::from_value(value.clone())?;
                        document.validate().map_err(crate::cad_ir::app_error)?;
                    }
                    revision.program = Some(serde_json::to_string(&value)?);
                }
            }
        }
        project.revisions.push(revision);
        project.current_revision = Some(next_id.to_owned());
    } else {
        project.current_revision = None;
    }
    project.updated_at = now.to_owned();
    project.thumbnail = None;
    project.thumbnail_revision = None;
    project.validate()?;
    let mut after = history.clone();
    after.cursor = cursor;
    after.head = project.current_revision.clone();
    after.generation = after
        .generation
        .checked_add(1)
        .ok_or_else(|| invalid("HISTORY_INVALID_STATE"))?;
    after.validate(&project)?;
    let prepared = PreparedHistory {
        before: history.clone(),
        after,
        before_project: serde_json::to_string(current)?,
        after_project: serde_json::to_string(&project)?,
        kind,
        target,
    };
    Ok((project, prepared))
}
