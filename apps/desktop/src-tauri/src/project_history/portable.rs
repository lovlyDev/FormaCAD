use super::{
    persistence,
    state::{same_snapshot, HistoryState},
};
use crate::{
    core::{AppError, Result},
    models::Project,
};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqliteConnection};
use std::collections::{HashMap, HashSet};

const MAX_HISTORY_BYTES: usize = 16 * 1024 * 1024;
const MAX_EVENTS: usize = 30000;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Archive {
    schema_version: u32,
    state: HistoryState,
    events: Vec<Event>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Event {
    kind: String,
    from_revision: Option<String>,
    to_revision: Option<String>,
    target_revision: Option<String>,
    created_at: String,
}

/// Export after record_commit, before committing the same SQL transaction.
/// Contains no project UUID, permission grants, paths, or provider credentials.
pub async fn export_history(
    connection: &mut SqliteConnection,
    project: &Project,
) -> Result<Vec<u8>> {
    let state = persistence::read(connection, project).await?;
    let rows = sqlx::query("SELECT kind,from_revision,to_revision,target_revision,created_at FROM project_history_events WHERE project_id=? ORDER BY id LIMIT 30001")
        .bind(&project.id).fetch_all(&mut *connection).await?;
    if rows.len() > MAX_EVENTS {
        return Err(AppError::Invalid("HISTORY_ARCHIVE_TOO_LARGE".into()));
    }
    let events = rows
        .into_iter()
        .map(|row| {
            Ok(Event {
                kind: row.try_get("kind")?,
                from_revision: row.try_get("from_revision")?,
                to_revision: row.try_get("to_revision")?,
                target_revision: row.try_get("target_revision")?,
                created_at: row.try_get("created_at")?,
            })
        })
        .collect::<std::result::Result<Vec<_>, sqlx::Error>>()?;
    let bytes = serde_json::to_vec(&Archive {
        schema_version: 1,
        state,
        events,
    })?;
    if bytes.len() > MAX_HISTORY_BYTES {
        return Err(AppError::Invalid("HISTORY_ARCHIVE_TOO_LARGE".into()));
    }
    Ok(bytes)
}

/// Import only into a new project's SQL transaction after its project row is
/// inserted. Original revision UUIDs must be preserved; project UUID may change.
/// Reject duplicate installation, future schemas, dangling IDs and cursor drift.
pub async fn import_history(
    connection: &mut SqliteConnection,
    project: &Project,
    bytes: &[u8],
) -> Result<()> {
    let archive = checked_archive(project, bytes)?;
    let existing: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM project_history WHERE project_id=?")
            .bind(&project.id)
            .fetch_one(&mut *connection)
            .await?;
    if existing != 0 {
        return Err(AppError::Invalid("HISTORY_ALREADY_EXISTS".into()));
    }
    sqlx::query("INSERT INTO project_history(project_id,state_json) VALUES(?,?)")
        .bind(&project.id)
        .bind(serde_json::to_string(&archive.state)?)
        .execute(&mut *connection)
        .await?;
    for event in archive.events {
        sqlx::query("INSERT INTO project_history_events(project_id,kind,from_revision,to_revision,target_revision,created_at) VALUES(?,?,?,?,?,?)")
            .bind(&project.id).bind(event.kind).bind(event.from_revision).bind(event.to_revision)
            .bind(event.target_revision).bind(event.created_at).execute(&mut *connection).await?;
    }
    Ok(())
}

/// Validate a folder/package archive before writing any imported project files.
pub fn validate_archive(project: &Project, bytes: &[u8]) -> Result<()> {
    checked_archive(project, bytes).map(|_| ())
}

fn checked_archive(project: &Project, bytes: &[u8]) -> Result<Archive> {
    project.validate()?;
    if bytes.len() > MAX_HISTORY_BYTES {
        return Err(AppError::Invalid("HISTORY_ARCHIVE_TOO_LARGE".into()));
    }
    let archive: Archive = serde_json::from_slice(bytes)?;
    if archive.schema_version != 1 || archive.events.len() > MAX_EVENTS {
        return Err(AppError::Invalid("HISTORY_INVALID_ARCHIVE".into()));
    }
    archive.state.validate(project)?;
    let revisions: HashMap<&str, _> = project
        .revisions
        .iter()
        .map(|revision| (revision.id.as_str(), revision))
        .collect();
    let mut previous: Option<&Option<String>> = None;
    let mut materialized = HashSet::new();
    for (index, event) in archive.events.iter().enumerate() {
        if !["edit", "undo", "redo"].contains(&event.kind.as_str())
            || [
                &event.from_revision,
                &event.to_revision,
                &event.target_revision,
            ]
            .into_iter()
            .flatten()
            .any(|id| !revisions.contains_key(id.as_str()))
            || event.created_at.is_empty()
            || event.created_at.len() > 80
            || previous.is_some_and(|previous| previous != &event.from_revision)
            || (event.kind == "edit"
                && (event.to_revision.is_none() || event.target_revision != event.to_revision))
            || (event.kind == "redo"
                && (event.to_revision.is_none() || event.target_revision.is_none()))
            || (event.kind == "undo"
                && event.to_revision.is_none() != event.target_revision.is_none())
        {
            return Err(AppError::Invalid("HISTORY_INVALID_ARCHIVE".into()));
        }
        if let Some(to) = &event.to_revision {
            let revision = revisions
                .get(to.as_str())
                .ok_or_else(|| AppError::Invalid("HISTORY_INVALID_ARCHIVE".into()))?;
            // An imported legacy project's first edit can represent an existing
            // parent chain. All subsequent events must create a fresh child.
            if !materialized.insert(to)
                || (index > 0 && revision.parent != event.from_revision)
                || (event.kind != "edit"
                    && (revision.parent != event.from_revision
                        || event.from_revision.as_ref() == Some(to)
                        || event
                            .target_revision
                            .as_ref()
                            .and_then(|target| revisions.get(target.as_str()))
                            .is_none_or(|target| !same_snapshot(target, revision))))
            {
                return Err(AppError::Invalid("HISTORY_INVALID_ARCHIVE".into()));
            }
        }
        previous = Some(&event.to_revision);
    }
    if previous.is_some_and(|head| head != &project.current_revision) {
        return Err(AppError::Invalid("HISTORY_INVALID_ARCHIVE".into()));
    }
    Ok(archive)
}
