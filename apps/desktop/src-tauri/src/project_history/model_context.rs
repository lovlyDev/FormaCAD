//! Borrowed model authority for cursor validation, independent of chat metadata.
use crate::models::{Parameters, Project, Revision};

pub(crate) struct HistoryRevision<'a> {
    pub id: &'a str,
    pub parent: Option<&'a str>,
    pub parameters: &'a Parameters,
    pub source: &'a Option<String>,
    pub preview: &'a Option<String>,
    pub program_base: &'a Option<String>,
    pub program: &'a Option<String>,
}

impl<'a> From<&'a Revision> for HistoryRevision<'a> {
    fn from(revision: &'a Revision) -> Self {
        Self {
            id: &revision.id,
            parent: revision.parent.as_deref(),
            parameters: &revision.parameters,
            source: &revision.source,
            preview: &revision.preview,
            program_base: &revision.program_base,
            program: &revision.program,
        }
    }
}

pub(crate) struct HistoryModelContext<'a> {
    pub current_revision: Option<&'a str>,
    pub revisions: Vec<HistoryRevision<'a>>,
}

impl<'a> From<&'a Project> for HistoryModelContext<'a> {
    fn from(project: &'a Project) -> Self {
        Self {
            current_revision: project.current_revision.as_deref(),
            revisions: project
                .revisions
                .iter()
                .map(HistoryRevision::from)
                .collect(),
        }
    }
}

/// Exactly the materialized-history equality, including authored program rules.
pub(crate) fn same_snapshot(baseline: &HistoryRevision<'_>, current: &HistoryRevision<'_>) -> bool {
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
        && canonical(baseline.program) == canonical(current.program)
}
