//! Pure history transition validation shared by storage backends.
use super::state::{HistoryState, PreparedHistory};
use crate::{
    core::{AppError, Result},
    models::Project,
};

/// Validated history payload. Construction stays inside this module so callers
/// cannot manufacture a successful transition by assembling its fields.
pub(crate) struct CommitHistory {
    state: HistoryState,
    event: Option<(&'static str, Option<String>)>,
}

impl CommitHistory {
    pub(crate) fn state(&self) -> &HistoryState {
        &self.state
    }

    pub(crate) fn event(&self) -> Option<(&'static str, Option<&str>)> {
        self.event
            .as_ref()
            .map(|(kind, target)| (*kind, target.as_deref()))
    }
}

/// Keep structural rejection ahead of SQL reads, as in the original writer.
pub(super) fn validate_before_read(
    old: Option<&Project>,
    new: &Project,
    transition: Option<&PreparedHistory>,
) -> Result<()> {
    new.validate()?;
    if transition.is_some() {
        old.ok_or_else(|| AppError::Invalid("HISTORY_INVALID_STATE".into()))?;
    } else if let Some(old) = old {
        if new.id != old.id
            || !new.revisions.starts_with(&old.revisions)
            || new.revisions.len() > old.revisions.len() + 1
            || old.files.iter().any(|file| !new.files.contains(file))
        {
            return Err(AppError::Invalid("HISTORY_IMMUTABLE".into()));
        }
    }
    Ok(())
}

/// Validate the same edit/metadata/undo/redo rules used by the SQL writer.
/// `None` persisted state preserves the legacy parent-chain fallback. This
/// function does not grant writer access, publish files, or mutate its inputs.
pub(crate) fn prepare_commit(
    old: Option<&Project>,
    persisted: Option<&HistoryState>,
    new: &Project,
    transition: Option<&PreparedHistory>,
) -> Result<CommitHistory> {
    validate_before_read(old, new, transition)?;
    let current = old
        .map(|old| -> Result<HistoryState> {
            let state = match persisted {
                Some(state) => state.clone(),
                None => HistoryState::from_project(old)?,
            };
            state.validate(old)?;
            Ok(state)
        })
        .transpose()?;
    let (state, event) = if let Some(intent) = transition {
        let old = old.ok_or_else(|| AppError::Invalid("HISTORY_INVALID_STATE".into()))?;
        let persisted = current.ok_or_else(|| AppError::Invalid("HISTORY_INVALID_STATE".into()))?;
        if persisted != intent.before
            || serde_json::to_string(old)? != intent.before_project
            || serde_json::to_string(new)? != intent.after_project
        {
            return Err(AppError::Invalid(
                "Project changed; retry the CAD edit".into(),
            ));
        }
        (
            intent.after.clone(),
            Some((intent.kind, intent.target.clone())),
        )
    } else if let Some(old) = old {
        let state = current.ok_or_else(|| AppError::Invalid("HISTORY_INVALID_STATE".into()))?;
        if new.current_revision == old.current_revision {
            if new.revisions != old.revisions {
                return Err(AppError::Invalid("HISTORY_INVALID_STATE".into()));
            }
            (state, None)
        } else {
            let added = new
                .revisions
                .last()
                .ok_or_else(|| AppError::Invalid("HISTORY_INVALID_STATE".into()))?;
            if new.revisions.len() != old.revisions.len() + 1
                || new.current_revision.as_ref() != Some(&added.id)
                || added.parent != old.current_revision
            {
                return Err(AppError::Invalid("HISTORY_INVALID_STATE".into()));
            }
            (
                state.edited(new)?,
                Some(("edit", new.current_revision.clone())),
            )
        }
    } else {
        let event = new
            .current_revision
            .as_ref()
            .map(|_| ("edit", new.current_revision.clone()));
        (HistoryState::from_project(new)?, event)
    };
    state.validate(new)?;
    Ok(CommitHistory { state, event })
}

#[cfg(test)]
#[path = "commit_tests.rs"]
mod tests;
