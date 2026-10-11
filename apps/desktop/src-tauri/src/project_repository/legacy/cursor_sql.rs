//! One coherent SQL read of project and typed cursor, without event traversal.
use super::sql;
use crate::core::{AppError, Result};
use sqlx::{Row, SqlitePool};
const MAX_STATE_BYTES: i64 = 16 * 1024 * 1024;

#[derive(PartialEq, Eq)]
pub(super) struct RawCursor {
    pub project: sql::RawSnapshot,
    pub state: Option<String>,
}

pub(super) async fn read(pool: &SqlitePool, id: &str) -> Result<RawCursor> {
    let mut transaction = pool.begin().await?;
    let project = sql::read_transaction(&mut transaction, id, false).await?;
    let state = if project.journal.is_none() {
        let row = sqlx::query("SELECT length(CAST(state_json AS BLOB)) AS size, CASE WHEN length(CAST(state_json AS BLOB))<=? THEN state_json END AS state_json FROM project_history WHERE project_id=?")
            .bind(MAX_STATE_BYTES).bind(id).fetch_optional(&mut *transaction).await?;
        match row {
            Some(row) => {
                let size: i64 = row.try_get("size")?;
                if size > MAX_STATE_BYTES {
                    return Err(AppError::Invalid("HISTORY_ARCHIVE_TOO_LARGE".into()));
                }
                Some(row.try_get("state_json")?)
            }
            None => None,
        }
    } else {
        None
    };
    transaction.rollback().await?;
    Ok(RawCursor { project, state })
}
