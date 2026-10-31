//! Typed cursor decoding independent of portable history event archives.
use super::HistoryState;
use crate::{core::Result, models::Project};

pub(crate) fn from_context_json(
    context: &super::model_context::HistoryModelContext<'_>,
    raw: Option<&str>,
) -> Result<HistoryState> {
    let state = match raw {
        Some(raw) => serde_json::from_str(raw)?,
        None => HistoryState::from_context(context)?,
    };
    state.validate_context(context)?;
    Ok(state)
}

pub(crate) fn from_json(project: &Project, raw: Option<&str>) -> Result<HistoryState> {
    let state = match raw {
        Some(raw) => serde_json::from_str(raw)?,
        None => HistoryState::from_project(project)?,
    };
    state.validate(project)?;
    Ok(state)
}
