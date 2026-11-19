//! Pure managed-project catalog projection, independent of intentional open/recovery.
//! Each ready row is an optimistic capture/recheck at that row's check point, not
//! an atomic multi-project snapshot or a transferable session/writer capability.
use super::backend::ProjectRepository;
use crate::{
    core::{AppError, AppState, Result},
    models::Project,
};
use serde::Serialize;
use sqlx::{sqlite::SqliteRow, Row};
use std::io::{self, Write};

/// Caller-selected limits, without product defaults. The byte limit applies to
/// the sum of serialized ready Project payloads, not bounded hints/envelopes or
/// the repository's transient capture/recheck working set.
#[derive(Clone, Copy)]
pub struct CatalogLimits {
    pub max_entries: usize,
    pub max_total_payload_bytes: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogReport {
    pub entries: Vec<CatalogEntry>,
    pub serialized_project_bytes: u64,
}

#[derive(Debug, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum CatalogEntry {
    Ready {
        row_key: String,
        project: Box<Project>,
    },
    Unavailable {
        row_key: String,
        index: IndexHints,
        issue: CatalogIssue,
    },
}

/// Display-only bounded app-index hints. Never use these as saved model data or
/// accept id_hint as a valid project identifier when project_id is absent.
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexHints {
    pub project_id: Option<String>,
    pub id_hint: Option<String>,
    pub name: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CatalogIssueStatus {
    RecoveryRequired,
    Changed,
    Unavailable,
    Limit,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogIssue {
    pub status: CatalogIssueStatus,
    pub code: String,
    pub details: String,
}

struct IndexedRow {
    row_key: i64,
    hints: IndexHints,
    payload_bytes: i64,
    error: Option<AppError>,
}

/// Enumerates every selected row. No get/reconcile, writer access, repair,
/// migration or payload fetch-all occurs here. A database/enumeration failure is
/// a top-level error; one project's bad saved state is an unavailable entry.
pub async fn list_managed(state: &AppState, limits: CatalogLimits) -> Result<CatalogReport> {
    let indexed = read_index(state, limits.max_entries).await?;
    project_entries(state, indexed, limits).await
}

async fn read_index(state: &AppState, maximum: usize) -> Result<Vec<IndexedRow>> {
    let count = maximum
        .checked_add(1)
        .and_then(|value| i64::try_from(value).ok())
        .ok_or_else(limit_error)?;
    // Every text cell is bounded in SQLite before it crosses the driver. Raw
    // payload length is only a preflight hint: capture still verifies real data.
    let rows = sqlx::query("SELECT rowid AS row_key, substr(id,1,37) AS id_hint, length(CAST(id AS BLOB)) AS id_bytes, CASE WHEN length(CAST(name AS BLOB))<=320 AND length(name)<=80 THEN name END AS name_hint, CASE WHEN length(CAST(updated_at AS BLOB))<=80 THEN updated_at END AS updated_hint, length(CAST(payload AS BLOB)) AS payload_bytes FROM projects ORDER BY updated_hint DESC,row_key DESC LIMIT ?")
        .bind(count).fetch_all(&state.pool).await?;
    if rows.len() > maximum {
        return Err(limit_error());
    }
    rows.into_iter().map(indexed_row).collect()
}

fn indexed_row(row: SqliteRow) -> Result<IndexedRow> {
    // rowid is SQLite's own numeric key, including for an invalid text UUID.
    let row_key = row.try_get("row_key")?;
    let mut hints = IndexHints::default();
    let decoded: Result<i64> = (|| {
        let id: String = row.try_get("id_hint")?;
        let id_bytes: i64 = row.try_get("id_bytes")?;
        hints.project_id =
            (id_bytes == 36 && crate::security::valid_id(&id).is_ok()).then(|| id.clone());
        hints.id_hint = Some(id);
        hints.name = row
            .try_get::<Option<String>, _>("name_hint")?
            .filter(|name| !name.trim().is_empty());
        hints.updated_at = row
            .try_get::<Option<String>, _>("updated_hint")?
            .filter(|date| chrono::DateTime::parse_from_rfc3339(date).is_ok());
        let bytes = row.try_get("payload_bytes")?;
        Ok(bytes)
    })();
    let (payload_bytes, error) = match decoded {
        Ok(bytes) => (bytes, None),
        Err(error) => (-1, Some(error)),
    };
    Ok(IndexedRow {
        row_key,
        hints,
        payload_bytes,
        error,
    })
}

async fn project_entries(
    state: &AppState,
    indexed: Vec<IndexedRow>,
    limits: CatalogLimits,
) -> Result<CatalogReport> {
    let mut entries = Vec::with_capacity(indexed.len());
    let mut used = 0u64;
    for row in indexed {
        let remaining = limits.max_total_payload_bytes - used;
        let result = project_entry(state, &row, remaining).await;
        match result {
            Ok((project, bytes)) => {
                used += bytes;
                entries.push(CatalogEntry::Ready {
                    row_key: format!("managed-index:{}", row.row_key),
                    project,
                });
            }
            Err(ProjectionFailure::Issue(issue)) => entries.push(CatalogEntry::Unavailable {
                row_key: format!("managed-index:{}", row.row_key),
                index: row.hints,
                issue,
            }),
            Err(ProjectionFailure::Global(error)) => return Err(error),
        }
    }
    Ok(CatalogReport {
        entries,
        serialized_project_bytes: used,
    })
}

enum ProjectionFailure {
    Issue(CatalogIssue),
    Global(AppError),
}

fn projection_failure(error: AppError) -> ProjectionFailure {
    match error {
        AppError::Database(_) => ProjectionFailure::Global(error),
        error => ProjectionFailure::Issue(issue_from_error(&error)),
    }
}

async fn project_entry(
    state: &AppState,
    row: &IndexedRow,
    remaining: u64,
) -> std::result::Result<(Box<Project>, u64), ProjectionFailure> {
    if let Some(error) = &row.error {
        return Err(ProjectionFailure::Issue(issue_from_error(error)));
    }
    let Some(id) = &row.hints.project_id else {
        return Err(ProjectionFailure::Issue(unavailable(
            "Invalid indexed project identifier",
        )));
    };
    if row.payload_bytes < 0 {
        return Err(ProjectionFailure::Issue(unavailable(
            "Invalid indexed payload length",
        )));
    }
    if row.payload_bytes as u64 > remaining {
        return Err(ProjectionFailure::Issue(limited("Stored payload preflight exceeds the remaining caller budget; this conservative check can refuse whitespace-heavy input before canonical serialization")));
    }
    let repository = ProjectRepository::managed(state, id).map_err(projection_failure)?;
    let snapshot = repository.capture().await.map_err(projection_failure)?;
    // A stale length hint cannot admit a larger actual response. This counts
    // serde's escaped bytes without materializing another giant JSON buffer.
    // A raced row may still use the existing bounded repository capture working
    // set: caller output limits are not a per-caller peak-memory guarantee.
    let bytes = serialized_size(snapshot.project(), remaining).map_err(projection_failure)?;
    repository
        .recheck(&snapshot)
        .await
        .map_err(projection_failure)?;
    // Clone only an admitted immutable Project value; never serialize or expose
    // RepositorySnapshot/token internals. This transiently retains the original
    // capture and one output clone, then drops the original authority.
    Ok((Box::new(snapshot.project().clone()), bytes))
}

fn issue_from_error(error: &AppError) -> CatalogIssue {
    let (status, code) = match error {
        AppError::Invalid(code) if code == "PROJECT_SNAPSHOT_RECOVERY_REQUIRED" => {
            (CatalogIssueStatus::RecoveryRequired, code.as_str())
        }
        AppError::Invalid(code)
            if code == "PROJECT_SNAPSHOT_CHANGED" || code == "PROJECT_SNAPSHOT_MISSING" =>
        {
            (CatalogIssueStatus::Changed, code.as_str())
        }
        AppError::Invalid(code) if code == "PROJECT_CATALOG_LIMIT" => {
            (CatalogIssueStatus::Limit, code.as_str())
        }
        _ => (
            CatalogIssueStatus::Unavailable,
            "PROJECT_CATALOG_UNAVAILABLE",
        ),
    };
    CatalogIssue {
        status,
        code: code.into(),
        details: error.to_string().chars().take(2048).collect(),
    }
}

fn unavailable(details: &str) -> CatalogIssue {
    CatalogIssue {
        status: CatalogIssueStatus::Unavailable,
        code: "PROJECT_CATALOG_UNAVAILABLE".into(),
        details: details.into(),
    }
}

fn limited(details: &str) -> CatalogIssue {
    CatalogIssue {
        status: CatalogIssueStatus::Limit,
        code: "PROJECT_CATALOG_LIMIT".into(),
        details: details.into(),
    }
}

fn limit_error() -> AppError {
    AppError::Invalid("PROJECT_CATALOG_LIMIT".into())
}

fn serialized_size(project: &Project, limit: u64) -> Result<u64> {
    let mut writer = CountingWriter {
        used: 0,
        limit,
        exceeded: false,
    };
    match serde_json::to_writer(&mut writer, project) {
        Ok(()) => Ok(writer.used),
        Err(_) if writer.exceeded => Err(limit_error()),
        Err(error) => Err(error.into()),
    }
}

struct CountingWriter {
    used: u64,
    limit: u64,
    exceeded: bool,
}
impl Write for CountingWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let Some(next) = self
            .used
            .checked_add(bytes.len() as u64)
            .filter(|next| *next <= self.limit)
        else {
            self.exceeded = true;
            return Err(io::Error::other("PROJECT_CATALOG_LIMIT"));
        };
        self.used = next;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
#[path = "catalog/tests.rs"]
mod tests;
