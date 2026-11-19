use super::state::{HistoryState, PreparedHistory};
use crate::{core::Result, models::Project};
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
    super::commit::validate_before_read(old, new, transition)?;
    let persisted = match old {
        Some(old) => Some(read(connection, old).await?),
        None => None,
    };
    let prepared = super::prepare_commit(old, persisted.as_ref(), new, transition)?;
    sqlx::query("INSERT INTO project_history(project_id,state_json) VALUES(?,?) ON CONFLICT(project_id) DO UPDATE SET state_json=excluded.state_json")
        .bind(&new.id).bind(serde_json::to_string(prepared.state())?).execute(&mut *connection).await?;
    if let Some((kind, target)) = prepared.event() {
        sqlx::query("INSERT INTO project_history_events(project_id,kind,from_revision,to_revision,target_revision,created_at) VALUES(?,?,?,?,?,?)")
            .bind(&new.id).bind(kind).bind(old.and_then(|project| project.current_revision.as_deref()))
            .bind(&new.current_revision).bind(target).bind(&new.updated_at).execute(&mut *connection).await?;
    }
    Ok(())
}
