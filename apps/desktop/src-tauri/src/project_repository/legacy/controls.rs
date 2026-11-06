//! Fast model controls with original SQL/session/metadata seals. O(payload) raw
//! transfer and hashing remain; this never claims O(1) IO or full CAS health.
use super::{
    changed, controls_folder::FolderSeal, controls_projection::Projection, cursor_sql,
    HistoryStateSnapshot,
};
use crate::{
    core::{AppError, AppState, Result},
    project_history::HistoryStatus,
    project_repository::session::ReadSession,
};

pub(crate) struct ControlsSnapshot {
    id: String,
    session: ReadSession,
    payload_hash: String,
    cursor_hash: Option<String>,
    folder: Option<FolderSeal>,
    fallback: Option<HistoryStateSnapshot>,
    status: HistoryStatus,
}

impl ControlsSnapshot {
    pub(crate) fn fallback(&self) -> Option<&HistoryStateSnapshot> {
        self.fallback.as_ref()
    }
    pub(crate) fn status(&self) -> HistoryStatus {
        HistoryStatus {
            can_undo: self.status.can_undo,
            can_redo: self.status.can_redo,
            expected_revision: self.status.expected_revision.clone(),
        }
    }

    pub(crate) async fn recheck(&self, state: &AppState) -> Result<()> {
        self.session.recheck(state, &self.id)?;
        if self.fallback.is_none() {
            self.recheck_folder(state)?;
        }
        let raw = cursor_sql::read(&state.pool, &self.id)
            .await
            .map_err(super::missing_to_changed)?;
        self.session.recheck(state, &self.id)?;
        if raw.project.journal.is_some()
            || crate::artifacts::digest(raw.project.project.as_bytes()) != self.payload_hash
            || raw
                .state
                .as_deref()
                .map(|raw| crate::artifacts::digest(raw.as_bytes()))
                != self.cursor_hash
        {
            return Err(changed());
        }
        if self.fallback.is_none() {
            self.recheck_folder(state)?;
        }
        self.session.recheck(state, &self.id)
    }

    fn recheck_folder(&self, state: &AppState) -> Result<()> {
        match &self.folder {
            Some(folder) => folder.recheck(state, &self.id),
            None => {
                if state
                    .root
                    .join(&self.id)
                    .join("manifest.json")
                    .try_exists()?
                {
                    return Err(changed());
                }
                Ok(())
            }
        }
    }
}

pub(crate) async fn capture_controls(state: &AppState, id: &str) -> Result<ControlsSnapshot> {
    crate::security::valid_id(id)?;
    let session = ReadSession::capture(state, id)?;
    let raw = cursor_sql::read(&state.pool, id).await?;
    session.recheck(state, id)?;
    if raw.project.journal.is_some() {
        return Err(AppError::Invalid(
            "PROJECT_SNAPSHOT_RECOVERY_REQUIRED".into(),
        ));
    }
    let payload_hash = crate::artifacts::digest(raw.project.project.as_bytes());
    let cursor_hash = raw
        .state
        .as_deref()
        .map(|raw| crate::artifacts::digest(raw.as_bytes()));
    let model: Projection = serde_json::from_str(&raw.project.project)?;
    if model.id != id {
        return Err(changed());
    }
    if model.supported() {
        model.validate()?;
    }
    let has_manifest = state.root.join(id).join("manifest.json").try_exists()?;
    let folder = if model.supported() && has_manifest {
        FolderSeal::capture(state, &model, &payload_hash, raw.project.project.len())?
    } else {
        None
    };
    let supported = model.supported() && (!has_manifest || folder.is_some());
    let (status, fallback) = if supported {
        let context = model.context();
        let history =
            crate::project_history::state_from_context_json(&context, raw.state.as_deref())?;
        (
            HistoryStatus {
                can_undo: history.can_undo(),
                can_redo: history.can_redo(),
                expected_revision: model.current_revision.clone(),
            },
            None,
        )
    } else {
        // Retain the outer opening and raw seals across this explicit legacy
        // fallback; a newly captured full snapshot cannot renew its authority.
        session.recheck(state, id)?;
        let full = super::capture_history_state(state, id).await?;
        if !full.matches_raw(&raw) {
            return Err(changed());
        }
        session.recheck(state, id)?;
        (
            HistoryStatus {
                can_undo: full.history_state.can_undo(),
                can_redo: full.history_state.can_redo(),
                expected_revision: full.project.current_revision.clone(),
            },
            Some(full),
        )
    };
    session.recheck(state, id)?;
    Ok(ControlsSnapshot {
        id: id.to_owned(),
        session,
        payload_hash,
        cursor_hash,
        folder,
        fallback,
        status,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project_repository::legacy::read_test_fixture as fixture;

    #[tokio::test]
    async fn full_fallback_cannot_replace_original_raw_authority_after_aba() {
        let temp = tempfile::tempdir().unwrap();
        let state = fixture::state(temp.path()).await;
        let original = fixture::project(&state, 16).await;
        let raw = cursor_sql::read(&state.pool, &original.id).await.unwrap();
        let mut changed_project = original.clone();
        changed_project.name.push('!');
        fixture::replace(&state, &changed_project, false).await;
        let full = super::super::capture_history_state(&state, &original.id)
            .await
            .unwrap();
        fixture::replace(&state, &original, false).await;
        assert!(!full.matches_raw(&raw));
        let restored = super::super::capture_history_state(&state, &original.id)
            .await
            .unwrap();
        assert!(restored.matches_raw(&raw));
    }
}
