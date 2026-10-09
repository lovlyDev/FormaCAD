use super::state::{HistoryState, PreparedHistory};
use crate::{
    core::{AppError, Result},
    models::Project,
};
use sqlx::{SqliteConnection, SqlitePool};

pub(super) async fn read(
    connection: &mut SqliteConnection,
    project: &Project,
) -> Result<HistoryState> {
    let raw: Option<String> =
        sqlx::query_scalar("SELECT state_json FROM project_history WHERE project_id=?")
            .bind(&project.id)
            .fetch_optional(connection)
            .await?;
    let state = match raw {
        Some(raw) => serde_json::from_str(&raw)?,
        None => HistoryState::from_project(project)?,
    };
    state.validate(project)?;
    Ok(state)
}

pub async fn load_state(pool: &SqlitePool, project: &Project) -> Result<HistoryState> {
    read(&mut *pool.acquire().await?, project).await
}

/// Called inside the SAME SQL transaction as the project upsert. The caller
/// must roll back on any failure and publish its staged folder only after commit.
pub async fn record_commit(
    connection: &mut SqliteConnection,
    old: Option<&Project>,
    new: &Project,
    transition: Option<&PreparedHistory>,
) -> Result<()> {
    new.validate()?;
    let (state, event) = if let Some(intent) = transition {
        let old = old.ok_or_else(|| AppError::Invalid("HISTORY_INVALID_STATE".into()))?;
        let persisted = read(connection, old).await?;
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
        if new.id != old.id
            || !new.revisions.starts_with(&old.revisions)
            || new.revisions.len() > old.revisions.len() + 1
            || old.files.iter().any(|file| !new.files.contains(file))
        {
            return Err(AppError::Invalid("HISTORY_IMMUTABLE".into()));
        }
        let state = read(connection, old).await?;
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
        if transition.is_some() {
            return Err(AppError::Invalid("HISTORY_INVALID_STATE".into()));
        }
        let event = new
            .current_revision
            .as_ref()
            .map(|_| ("edit", new.current_revision.clone()));
        (HistoryState::from_project(new)?, event)
    };
    state.validate(new)?;
    sqlx::query("INSERT INTO project_history(project_id,state_json) VALUES(?,?) ON CONFLICT(project_id) DO UPDATE SET state_json=excluded.state_json")
        .bind(&new.id).bind(serde_json::to_string(&state)?).execute(&mut *connection).await?;
    if let Some((kind, target)) = event {
        sqlx::query("INSERT INTO project_history_events(project_id,kind,from_revision,to_revision,target_revision,created_at) VALUES(?,?,?,?,?,?)")
            .bind(&new.id).bind(kind).bind(old.and_then(|project| project.current_revision.as_deref()))
            .bind(&new.current_revision).bind(target).bind(&new.updated_at).execute(&mut *connection).await?;
    }
    Ok(())
}
