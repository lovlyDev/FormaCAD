//! Full bounded archive validation. Metadata readers never call this route.
use super::super::{corrupt, limited};
use crate::{core::Result, models::Project};
use sqlx::SqliteConnection;

pub(super) async fn archive(
    connection: &mut SqliteConnection,
    project: &Project,
) -> Result<Vec<u8>> {
    // Bound event cells before shared export crosses the driver. Invalid foreign
    // rows are rejected, rather than hidden by export's project_id predicate.
    let invalid_events: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM project_history_events WHERE project_id IS NULL OR project_id<>? OR length(CAST(project_id AS BLOB))<>36 OR length(CAST(kind AS BLOB))>4 OR length(CAST(created_at AS BLOB))>80 OR (from_revision IS NOT NULL AND length(CAST(from_revision AS BLOB))<>36) OR (to_revision IS NOT NULL AND length(CAST(to_revision AS BLOB))<>36) OR (target_revision IS NOT NULL AND length(CAST(target_revision AS BLOB))<>36)")
        .bind(&project.id).fetch_one(&mut *connection).await?;
    if invalid_events != 0 {
        return Err(corrupt());
    }
    let event_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM project_history_events")
        .fetch_one(&mut *connection)
        .await?;
    if event_count > 30_000 {
        return Err(limited());
    }
    let history = crate::project_history::export_history(connection, project).await?;
    crate::project_history::validate_archive(project, &history)?;
    Ok(history)
}
