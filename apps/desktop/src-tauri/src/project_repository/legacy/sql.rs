use super::changed;
use crate::{
    core::{AppError, Result},
    models::Project,
};
use sqlx::{Row, SqliteConnection, SqlitePool};
// Legacy SQL projects may include 100 MiB embedded data; portable metadata's
// smaller limit must not reject ordinary SQL-only embedded STEP projects.
const MAX_PROJECT_BYTES: i64 = 128 * 1024 * 1024;
const MAX_HISTORY_BYTES: i64 = 16 * 1024 * 1024;
type Event = (
    i64,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
);
#[derive(PartialEq, Eq)]
pub(super) struct RawSnapshot {
    pub project: String,
    pub journal: Option<()>,
    pub history: Option<Vec<u8>>,
    history_raw: Option<String>,
    events: Vec<Event>,
}
pub(super) async fn read(pool: &SqlitePool, id: &str, with_history: bool) -> Result<RawSnapshot> {
    let mut transaction = pool.begin().await?;
    let raw = read_transaction(&mut transaction, id, with_history).await?;
    transaction.rollback().await?;
    Ok(raw)
}

async fn read_transaction(
    connection: &mut SqliteConnection,
    id: &str,
    with_history: bool,
) -> Result<RawSnapshot> {
    let row=sqlx::query("SELECT length(CAST(payload AS BLOB)) AS size, CASE WHEN length(CAST(payload AS BLOB))<=? THEN payload END AS payload FROM projects WHERE id=?")
        .bind(MAX_PROJECT_BYTES).bind(id).fetch_optional(&mut *connection).await?
        .ok_or_else(||AppError::Invalid("PROJECT_SNAPSHOT_MISSING".into()))?;
    let size: i64 = row.try_get("size")?;
    if size > MAX_PROJECT_BYTES {
        return Err(AppError::Invalid(
            "Project storage metadata exceeds its limit".into(),
        ));
    }
    let project: String = row.try_get("payload")?;
    // Pending journals are refused: never load their untrusted manifest cells.
    let pending: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM project_storage_commits WHERE project_id=?)",
    )
    .bind(id)
    .fetch_one(&mut *connection)
    .await?;
    let journal = pending.then_some(());
    let mut raw = RawSnapshot {
        project,
        journal,
        history: None,
        history_raw: None,
        events: vec![],
    };
    if with_history && raw.journal.is_none() {
        let project: Project = serde_json::from_str(&raw.project)?;
        project.validate()?;
        if project.id != id {
            return Err(changed());
        }
        let row=sqlx::query("SELECT length(CAST(state_json AS BLOB)) AS size, CASE WHEN length(CAST(state_json AS BLOB))<=? THEN state_json END AS state_json FROM project_history WHERE project_id=?")
            .bind(MAX_HISTORY_BYTES).bind(id).fetch_optional(&mut *connection).await?;
        if let Some(row) = row {
            let size: i64 = row.try_get("size")?;
            if size > MAX_HISTORY_BYTES {
                return Err(AppError::Invalid("HISTORY_ARCHIVE_TOO_LARGE".into()));
            }
            raw.history_raw = Some(row.try_get("state_json")?);
        }
        // Bound text cells before allocating rows or serializing the archive.
        let event_bytes:i64=sqlx::query_scalar("SELECT COALESCE(SUM(length(CAST(kind AS BLOB))+COALESCE(length(CAST(from_revision AS BLOB)),0)+COALESCE(length(CAST(to_revision AS BLOB)),0)+COALESCE(length(CAST(target_revision AS BLOB)),0)+length(CAST(created_at AS BLOB))+64),0) FROM (SELECT kind,from_revision,to_revision,target_revision,created_at FROM project_history_events WHERE project_id=? ORDER BY id LIMIT 30001)")
            .bind(id).fetch_one(&mut *connection).await?;
        if event_bytes > MAX_HISTORY_BYTES {
            return Err(AppError::Invalid("HISTORY_ARCHIVE_TOO_LARGE".into()));
        }
        raw.events=sqlx::query_as("SELECT id,kind,from_revision,to_revision,target_revision,created_at FROM project_history_events WHERE project_id=? ORDER BY id LIMIT 30001")
            .bind(id).fetch_all(&mut *connection).await?;
        if raw.events.len() > 30000 {
            return Err(AppError::Invalid("HISTORY_ARCHIVE_TOO_LARGE".into()));
        }
        let history = crate::project_history::export_history(connection, &project).await?;
        crate::project_history::validate_archive(&project, &history)?;
        raw.history = Some(history);
    }
    Ok(raw)
}

#[cfg(test)]
#[path = "sql_tests.rs"]
mod tests;
