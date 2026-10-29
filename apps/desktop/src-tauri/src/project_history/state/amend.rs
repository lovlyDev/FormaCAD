use super::{invalid, HistoryState, PreparedHistory};
use crate::{core::Result, models::Project};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AmendCapture {
    pub project_id: String,
    pub expected_revision: String,
    pub transaction_id: String,
    pub generation: u64,
    pub base_revision: String,
}

impl HistoryState {
    /// Only the last committed edit can be amended. Materialized undo/redo heads cannot.
    pub(crate) fn capture_amend(&self, current: &Project) -> Result<Option<AmendCapture>> {
        self.validate(current)?;
        if self.cursor < 2
            || self.cursor + 1 != self.entries.len()
            || self.head != self.entries[self.cursor]
        {
            return Ok(None);
        }
        let (Some(head), Some(base)) = (&self.head, &self.entries[self.cursor - 1]) else {
            return Ok(None);
        };
        Ok(Some(AmendCapture {
            project_id: current.id.clone(),
            expected_revision: head.clone(),
            transaction_id: head.clone(),
            generation: self.generation,
            base_revision: base.clone(),
        }))
    }
}

/// Replace one logical Undo slot; all previous revisions/artifacts remain immutable.
pub(crate) fn prepare_amend(
    current: &Project,
    next: &Project,
    history: &HistoryState,
    capture: &AmendCapture,
) -> Result<PreparedHistory> {
    current.validate()?;
    next.validate()?;
    if history.capture_amend(current)?.as_ref() != Some(capture)
        || next.id != current.id
        || !next.revisions.starts_with(&current.revisions)
        || next.revisions.len() != current.revisions.len() + 1
        || current.files.iter().any(|file| !next.files.contains(file))
    {
        return Err(invalid("AMEND_STALE"));
    }
    let revision = next
        .revisions
        .last()
        .ok_or_else(|| invalid("AMEND_STALE"))?;
    if next.current_revision.as_ref() != Some(&revision.id)
        || revision.parent != current.current_revision
        || current.revisions.iter().any(|old| old.id == revision.id)
    {
        return Err(invalid("AMEND_STALE"));
    }
    let mut after = history.clone();
    after.entries[after.cursor] = Some(revision.id.clone());
    after.head = Some(revision.id.clone());
    after.generation = after
        .generation
        .checked_add(1)
        .ok_or_else(|| invalid("HISTORY_INVALID_STATE"))?;
    after.validate(next)?;
    Ok(PreparedHistory {
        before: history.clone(),
        after,
        before_project: serde_json::to_string(current)?,
        after_project: serde_json::to_string(next)?,
        // Immutable revisions remain fresh children; only the logical Undo slot is replaced.
        kind: "edit",
        target: next.current_revision.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Parameters, Revision};
    fn id(label: &str) -> String {
        uuid::Uuid::from_u128(
            label
                .bytes()
                .fold(0u128, |value, byte| value * 256 + u128::from(byte)),
        )
        .to_string()
    }
    fn project() -> Project {
        Project {
            schema_version: 1,
            id: uuid::Uuid::new_v4().to_string(),
            name: "Fixture".into(),
            units: "mm".into(),
            agent: "codex".into(),
            pinned: false,
            thumbnail: None,
            thumbnail_revision: None,
            created_at: "2026-10-29T00:00:00Z".into(),
            updated_at: "2026-10-29T00:00:00Z".into(),
            revisions: vec![],
            current_revision: None,
            messages: vec![],
            files: vec![],
            exports: vec![],
        }
    }
    fn revision(current: &Project, id: &str, parent: Option<String>) -> Project {
        let mut next = current.clone();
        next.revisions.push(Revision {
            id: self::id(id),
            parent: parent.map(|parent| self::id(&parent)),
            created_at: "2026-10-29T00:00:00Z".into(),
            prompt: "Geometry edit".into(),
            parameters: Parameters {
                kind: "blank".into(),
                width: 20.,
                depth: 10.,
                height: 5.,
                thickness: 2.,
                hole_diameter: 1.,
                holes: 0,
            },
            source: None,
            preview: None,
            program: None,
            program_base: None,
        });
        next.current_revision = Some(self::id(id));
        next
    }
    #[test]
    fn repeated_amend_keeps_one_undo_slot_and_immutable_snapshots() {
        let first = revision(&project(), "base", None);
        let moved = revision(&first, "move", Some("base".into()));
        let history = HistoryState::from_project(&moved).unwrap();
        let capture = history.capture_amend(&moved).unwrap().unwrap();
        let amended = revision(&moved, "amend1", Some("move".into()));
        let intent = prepare_amend(&moved, &amended, &history, &capture).unwrap();
        assert_eq!(intent.after.entries.len(), history.entries.len());
        assert_eq!(intent.after.entries.last(), Some(&Some(id("amend1"))));
        assert!(amended.revisions.starts_with(&moved.revisions));
        let second_capture = intent.after.capture_amend(&amended).unwrap().unwrap();
        let again = revision(&amended, "amend2", Some("amend1".into()));
        let second = prepare_amend(&amended, &again, &intent.after, &second_capture).unwrap();
        assert_eq!(second.after.entries.len(), 3);
        let (undone, _) = super::super::prepare(
            &again,
            &second.after,
            super::super::Direction::Undo,
            Some(id("amend2").as_str()),
            &id("undo"),
            "2026-10-29T00:00:00Z",
        )
        .unwrap();
        // Undo materializes the base snapshot; it does not visit the original 10 mm or first amendment.
        assert_eq!(
            undone.revisions.last().unwrap().prompt,
            format!("history:undo:{}", id("base"))
        );
        assert_eq!(moved.current_revision, Some(id("move")));
    }
    #[test]
    fn stale_generation_or_intervening_history_rejects_replacement() {
        let base = revision(&project(), "base", None);
        let moved = revision(&base, "move", Some("base".into()));
        let history = HistoryState::from_project(&moved).unwrap();
        let mut capture = history.capture_amend(&moved).unwrap().unwrap();
        let next = revision(&moved, "amend", Some("move".into()));
        capture.generation += 1;
        assert_eq!(
            prepare_amend(&moved, &next, &history, &capture)
                .err()
                .unwrap()
                .to_string(),
            "AMEND_STALE"
        );
        let (undone, intent) = super::super::prepare(
            &moved,
            &history,
            super::super::Direction::Undo,
            Some(id("move").as_str()),
            &id("undo"),
            "2026-10-29T00:00:00Z",
        )
        .unwrap();
        assert!(intent.after.capture_amend(&undone).unwrap().is_none());
        let invalid = revision(&moved, "amend", Some("base".into()));
        let capture = history.capture_amend(&moved).unwrap().unwrap();
        assert!(prepare_amend(&moved, &invalid, &history, &capture).is_err());
    }
}
