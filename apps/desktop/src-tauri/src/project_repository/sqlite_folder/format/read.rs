//! Fixed bounded SELECTs only. Never repair, migrate, enable a writer or touch CAS.
use super::*;
use sqlx::{sqlite::SqliteRow, Connection, Row};

pub(super) async fn read(connection: &mut SqliteConnection) -> Result<Snapshot> {
    let mut transaction = connection.begin_with("BEGIN DEFERRED").await?;
    let result = within(&mut transaction).await;
    transaction.rollback().await?;
    result
}

pub(super) async fn validate_schema(connection: &mut SqliteConnection) -> Result<()> {
    let expected = expected_schema().await?;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_schema")
        .fetch_one(&mut *connection)
        .await?;
    if count != expected.len() as i64 {
        return Err(unsupported());
    }
    let rows = sqlx::query("SELECT CASE WHEN length(CAST(type AS BLOB))<=16 THEN type END AS type,CASE WHEN length(CAST(name AS BLOB))<=128 THEN name END AS name,CASE WHEN length(CAST(tbl_name AS BLOB))<=128 THEN tbl_name END AS tbl_name,CASE WHEN length(CAST(sql AS BLOB))<=65536 THEN sql END AS definition FROM sqlite_schema ORDER BY name")
        .fetch_all(&mut *connection).await?;
    let mut actual = Vec::with_capacity(rows.len());
    for row in rows {
        let kind = text(&row, "type")?;
        let name = text(&row, "name")?;
        let table = text(&row, "tbl_name")?;
        let definition: Option<String> = row.try_get("definition")?;
        actual.push((
            kind,
            name,
            table,
            definition.map(|sql| sql.trim().to_owned()),
        ));
    }
    if &actual != expected {
        return Err(unsupported());
    }
    Ok(())
}

fn text(row: &SqliteRow, field: &str) -> Result<String> {
    row.try_get::<Option<String>, _>(field)?.ok_or_else(corrupt)
}
fn optional_uuid(row: &SqliteRow, field: &str) -> Result<Option<String>> {
    let value: Option<String> = row.try_get(field)?;
    if let Some(value) = &value {
        crate::security::valid_id(value).map_err(|_| corrupt())?;
    }
    Ok(value)
}
fn checked_hash(value: String) -> Result<String> {
    if !crate::artifacts::valid_digest(&value) {
        return Err(corrupt());
    }
    Ok(value)
}

pub(super) async fn within(connection: &mut SqliteConnection) -> Result<Snapshot> {
    let application: i64 = sqlx::query_scalar("PRAGMA application_id")
        .fetch_one(&mut *connection)
        .await?;
    let version: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(&mut *connection)
        .await?;
    if application != APPLICATION_ID || version != USER_VERSION {
        return Err(unsupported());
    }
    validate_schema(connection).await?;
    let format_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM folder_format")
        .fetch_one(&mut *connection)
        .await?;
    let state_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM repository_state")
        .fetch_one(&mut *connection)
        .await?;
    if format_count != 1 || state_count != 1 {
        return Err(corrupt());
    }
    let format_rows = sqlx::query("SELECT CASE WHEN length(CAST(marker AS BLOB))<=64 THEN marker END AS marker,CASE WHEN length(schema_sha256)=64 THEN schema_sha256 END AS schema_sha256,CASE WHEN length(schema_fingerprint)=64 THEN schema_fingerprint END AS schema_fingerprint FROM folder_format WHERE singleton=1")
        .fetch_all(&mut *connection).await?;
    if format_rows.len() != 1 {
        return Err(corrupt());
    }
    let marker = &format_rows[0];
    if text(marker, "marker")? != FORMAT_MARKER
        || text(marker, "schema_sha256")? != schema_hash()
        || text(marker, "schema_fingerprint")? != schema_fingerprint().await?
    {
        return Err(unsupported());
    }
    let state_rows = sqlx::query("SELECT CASE WHEN length(CAST(repository_id AS BLOB))=36 THEN repository_id END AS repository_id,CASE WHEN length(CAST(project_id AS BLOB))=36 THEN project_id END AS project_id,CASE WHEN length(CAST(head_commit_id AS BLOB))=36 THEN head_commit_id END AS head_commit_id,sequence FROM repository_state WHERE singleton=1")
        .fetch_all(&mut *connection).await?;
    if state_rows.len() != 1 {
        return Err(corrupt());
    }
    let state_row = &state_rows[0];
    let repository_id = text(state_row, "repository_id")?;
    let project_id = text(state_row, "project_id")?;
    let commit_id = text(state_row, "head_commit_id")?;
    for id in [&repository_id, &project_id, &commit_id] {
        crate::security::valid_id(id).map_err(|_| corrupt())?;
    }
    if repository_id == project_id {
        return Err(corrupt());
    }
    let sequence =
        u64::try_from(state_row.try_get::<i64, _>("sequence")?).map_err(|_| corrupt())?;
    if sequence == 0 || sequence > MAX_COMMITS {
        return Err(limited());
    }
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM projects")
        .fetch_one(&mut *connection)
        .await?;
    if count != 1 {
        return Err(corrupt());
    }
    let row = sqlx::query("SELECT CASE WHEN length(CAST(payload AS BLOB))<=? THEN payload END AS payload,CASE WHEN length(CAST(name AS BLOB))<=320 THEN name END AS name,CASE WHEN length(CAST(updated_at AS BLOB))<=80 THEN updated_at END AS updated_at FROM projects WHERE id=?")
        .bind(MAX_PROJECT_BYTES).bind(&project_id).fetch_optional(&mut *connection).await?.ok_or_else(corrupt)?;
    let payload = text(&row, "payload")?;
    let project: Project = serde_json::from_str(&payload)?;
    normalized_project(&project)?;
    let revision_ids: std::collections::HashSet<_> = project
        .revisions
        .iter()
        .map(|revision| revision.id.as_str())
        .collect();
    if project.id != project_id
        || project.name != text(&row, "name")?
        || project.updated_at != text(&row, "updated_at")?
        || serde_json::to_string(&project)? != payload
    {
        return Err(corrupt());
    }
    let cursor_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM project_history")
        .fetch_one(&mut *connection)
        .await?;
    if cursor_count != 1 {
        return Err(corrupt());
    }
    let cursor: Option<String> = sqlx::query_scalar("SELECT CASE WHEN length(CAST(state_json AS BLOB))<=? THEN state_json END FROM project_history WHERE project_id=?")
        .bind(MAX_HISTORY_BYTES).bind(&project_id).fetch_optional(&mut *connection).await?.flatten();
    let cursor = cursor.ok_or_else(corrupt)?;
    let history_state = crate::project_history::state_from_json(&project, Some(&cursor))?;
    if serde_json::to_string(&history_state)? != cursor {
        return Err(corrupt());
    }
    // Bound event cells before shared export crosses the driver. Invalid foreign
    // rows are rejected, rather than hidden by export's project_id predicate.
    let invalid_events: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM project_history_events WHERE project_id IS NULL OR project_id<>? OR length(CAST(project_id AS BLOB))<>36 OR length(CAST(kind AS BLOB))>4 OR length(CAST(created_at AS BLOB))>80 OR (from_revision IS NOT NULL AND length(CAST(from_revision AS BLOB))<>36) OR (to_revision IS NOT NULL AND length(CAST(to_revision AS BLOB))<>36) OR (target_revision IS NOT NULL AND length(CAST(target_revision AS BLOB))<>36)")
        .bind(&project_id).fetch_one(&mut *connection).await?;
    if invalid_events != 0 {
        return Err(corrupt());
    }
    let event_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM project_history_events")
        .fetch_one(&mut *connection)
        .await?;
    if event_count > 30_000 {
        return Err(limited());
    }
    let history = crate::project_history::export_history(connection, &project).await?;
    crate::project_history::validate_archive(&project, &history)?;
    let commit_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM folder_commits")
        .fetch_one(&mut *connection)
        .await?;
    if commit_count != sequence as i64 {
        return Err(corrupt());
    }
    let rows = sqlx::query("SELECT CASE WHEN length(CAST(commit_id AS BLOB))=36 THEN commit_id END AS commit_id,CASE WHEN parent_commit_id IS NULL THEN NULL WHEN length(CAST(parent_commit_id AS BLOB))=36 THEN parent_commit_id ELSE '' END AS parent_commit_id,sequence,CASE WHEN revision_id IS NULL THEN NULL WHEN length(CAST(revision_id AS BLOB))=36 THEN revision_id ELSE '' END AS revision_id,CASE WHEN length(project_sha256)=64 THEN project_sha256 END AS project_sha256,CASE WHEN length(cursor_sha256)=64 THEN cursor_sha256 END AS cursor_sha256,CASE WHEN length(history_sha256)=64 THEN history_sha256 END AS history_sha256,CASE WHEN length(seal_sha256)=64 THEN seal_sha256 END AS seal_sha256 FROM folder_commits ORDER BY sequence")
        .fetch_all(&mut *connection).await?;
    let mut previous = None;
    let mut head = None;
    for (index, row) in rows.into_iter().enumerate() {
        let record = CommitRecord {
            id: text(&row, "commit_id")?,
            parent: optional_uuid(&row, "parent_commit_id")?,
            sequence: u64::try_from(row.try_get::<i64, _>("sequence")?).map_err(|_| corrupt())?,
            revision: optional_uuid(&row, "revision_id")?,
            project_hash: checked_hash(text(&row, "project_sha256")?)?,
            cursor_hash: checked_hash(text(&row, "cursor_sha256")?)?,
            history_hash: checked_hash(text(&row, "history_sha256")?)?,
            seal: checked_hash(text(&row, "seal_sha256")?)?,
        };
        crate::security::valid_id(&record.id).map_err(|_| corrupt())?;
        if record.sequence != index as u64 + 1
            || record.parent != previous
            || record.seal != record_seal(&repository_id, &project_id, &record)?
            || record
                .revision
                .as_ref()
                .is_some_and(|id| !revision_ids.contains(id.as_str()))
        {
            return Err(corrupt());
        }
        previous = Some(record.id.clone());
        head = Some(record);
    }
    let head = head.ok_or_else(corrupt)?;
    if head.id != commit_id
        || head.sequence != sequence
        || head.revision != project.current_revision
        || head.project_hash != crate::artifacts::digest(payload.as_bytes())
        || head.cursor_hash != crate::artifacts::digest(cursor.as_bytes())
        || head.history_hash != crate::artifacts::digest(&history)
    {
        return Err(corrupt());
    }
    Ok(Snapshot {
        expectation: Expectation {
            repository_id,
            project_id,
            commit_id,
            sequence,
            revision: project.current_revision.clone(),
        },
        project,
        history_state,
        history,
        saved_state: SavedStateSeals {
            project: head.project_hash,
            cursor: head.cursor_hash,
            history: head.history_hash,
        },
    })
}
