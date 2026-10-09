//! Revision-scoped viewport selection. This is context, never a persistent topology reference.
use crate::core::{AppError, Result};
use serde::{Deserialize, Serialize};
mod topology;
pub mod trusted;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SelectionContext {
    pub body_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub face_ordinal: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edge_ordinal: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub semantic_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub topology_ref: Option<crate::cad_ir::TopologyReference>,
    pub revision_id: String,
}

impl SelectionContext {
    pub fn validate(&self, current_revision: Option<&str>) -> Result<()> {
        if current_revision != Some(&self.revision_id)
            || match (self.face_ordinal, self.edge_ordinal) {
                (Some(value), None) | (None, Some(value)) => value == 0 || value > 100_000,
                _ => true,
            }
            || self.body_id.is_empty()
            || self.topology_ref.as_ref().is_some_and(|reference| {
                !reference.is_valid()
                    || match reference.kind {
                        crate::cad_ir::TopologyKind::Edge => self.edge_ordinal.is_none(),
                        crate::cad_ir::TopologyKind::Face => self.face_ordinal.is_none(),
                    }
            })
            || self.semantic_key.as_deref().is_some_and(|key| {
                self.edge_ordinal.is_none() || !crate::cad_ir::topology::valid_box_edge_key(key)
            })
            || self.body_id.len() > 80
            || !self
                .body_id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
        {
            return Err(AppError::Invalid(
                "Selected CAD geometry is stale or invalid".into(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::SelectionContext;

    #[test]
    fn stale_selection_cannot_be_sent_to_an_agent() {
        let selected = SelectionContext {
            body_id: "housing".into(),
            face_ordinal: Some(3),
            edge_ordinal: None,
            semantic_key: None,
            topology_ref: None,
            revision_id: "revision_a".into(),
        };
        assert!(selected.validate(Some("revision_a")).is_ok());
        assert!(selected.validate(Some("revision_b")).is_err());
        assert!(selected.validate(None).is_err());
    }

    #[test]
    fn edge_selection_requires_one_valid_revision_scoped_ordinal() {
        let selected = SelectionContext {
            body_id: "housing".into(),
            face_ordinal: None,
            edge_ordinal: Some(12),
            semantic_key: None,
            topology_ref: None,
            revision_id: "revision_a".into(),
        };
        assert!(selected.validate(Some("revision_a")).is_ok());
        assert!(selected.validate(Some("revision_b")).is_err());
        let mut invalid = selected;
        invalid.face_ordinal = Some(2);
        assert!(invalid.validate(Some("revision_a")).is_err());
    }
}
