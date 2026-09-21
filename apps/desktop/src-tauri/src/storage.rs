use crate::core::{AppError, Result};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
    Row, SqlitePool,
};
use std::path::Path;
pub async fn open(path: &Path) -> Result<SqlitePool> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(options)
        .await?;
    let status: String = sqlx::query_scalar("PRAGMA quick_check")
        .fetch_one(&pool)
        .await?;
    if status != "ok" {
        return Err(AppError::Invalid(
            "Database integrity check failed. Preserve this database and restore a backup.".into(),
        ));
    }
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|e| AppError::Invalid(format!("Database migration failed: {e}")))?;
    let interrupted = sqlx::query("SELECT project_id FROM agent_sessions WHERE status='running'")
        .fetch_all(&pool)
        .await?;
    for row in interrupted {
        let id: String = row.try_get("project_id")?;
        sqlx::query("INSERT INTO activities(project_id,kind,detail,created_at) VALUES(?,'recovery','Previous agent session was interrupted. The last saved revision was preserved.',?)").bind(id).bind(chrono::Utc::now().to_rfc3339()).execute(&pool).await?;
    }
    sqlx::query("UPDATE agent_sessions SET status='interrupted' WHERE status='running'")
        .execute(&pool)
        .await?;
    Ok(pool)
}
