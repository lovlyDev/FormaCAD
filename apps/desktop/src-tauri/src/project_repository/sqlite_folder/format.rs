//! Production FCP1 SQL format, distinct from the FCB1 comparison prototype.
//! Own immutable schema lineage. History tables deliberately retain the shared
//! record_commit/export_history contract; app migration runners are never used.
//! Caller owns filesystem identity, immutable CAS publication and writer authority.
//! Commit records seal digests; they do not retain every past metadata payload
//! or constitute signed provenance against an attacker rewriting the whole DB.
use crate::{
    core::{AppError, Result},
    models::Project,
    project_history::{HistoryState, PreparedHistory},
};
use sqlx::{Connection, Row, SqliteConnection};

mod compatibility;
#[cfg(test)]
mod metadata_tests;
mod outcome;
mod read;
mod write;
#[derive(Debug, PartialEq, Eq)]
pub(super) enum CommitPresence {
    Present { is_head: bool },
    Absent,
}

pub(super) async fn resolve_commit(
    connection: &mut SqliteConnection,
    source: &Expectation,
    commit_id: &str,
) -> Result<CommitPresence> {
    outcome::resolve_commit(connection, source, commit_id).await
}

pub(super) const APPLICATION_ID: i64 = 0x4643_5031;
pub(super) const USER_VERSION: i64 = 1;
pub(super) const FORMAT_MARKER: &str = "FormaCAD.FolderProject";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ReadScope {
    Metadata,
    Complete,
}
const SCHEMA: &str = include_str!("format/schema.sql");
const MAX_PROJECT_BYTES: i64 = 128 * 1024 * 1024;
const MAX_HISTORY_BYTES: i64 = 16 * 1024 * 1024;
const MAX_COMMITS: u64 = 30_000;

/// SQL expectation only: no filesystem identity, session, lease or UI authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Expectation {
    repository_id: String,
    project_id: String,
    commit_id: String,
    sequence: u64,
    revision: Option<String>,
}
impl Expectation {
    pub(super) fn repository_id(&self) -> &str {
        &self.repository_id
    }
    pub(super) fn project_id(&self) -> &str {
        &self.project_id
    }
    pub(super) fn commit_id(&self) -> &str {
        &self.commit_id
    }
    pub(super) fn sequence(&self) -> u64 {
        self.sequence
    }
    pub(super) fn revision(&self) -> Option<&str> {
        self.revision.as_deref()
    }
}

#[derive(Clone, Debug)]
pub(super) struct Snapshot {
    expectation: Expectation,
    project: Project,
    history_state: HistoryState,
    history: Option<Vec<u8>>,
    saved_state: SavedStateSeals,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct SavedStateSeals {
    project: String,
    cursor: String,
    history: String,
}

/// Small process-local comparison value; no project/history payload or authority.
#[cfg(any(target_os = "linux", windows))]
#[derive(Clone, PartialEq, Eq)]
pub(super) struct SavedStateStamp {
    expectation: Expectation,
    seals: SavedStateSeals,
}
impl Snapshot {
    #[cfg(any(target_os = "linux", windows))]
    pub(super) fn stamp(&self) -> SavedStateStamp {
        SavedStateStamp {
            expectation: self.expectation.clone(),
            seals: self.saved_state.clone(),
        }
    }

    pub(super) fn project(&self) -> &Project {
        &self.project
    }
    pub(super) fn history(&self) -> Option<&[u8]> {
        self.history.as_deref()
    }
    pub(super) fn into_history(self) -> Option<Vec<u8>> {
        self.history
    }
    pub(super) fn history_state(&self) -> &HistoryState {
        &self.history_state
    }
    pub(super) fn expectation(&self) -> &Expectation {
        &self.expectation
    }
    /// Compare the validated saved bytes as well as SQL commit identity. This
    /// catches same-ID metadata tampering even if its stored checksums were
    /// recomputed, without serializing another full Project during recheck.
    pub(super) fn same_saved_state(&self, other: &Self) -> bool {
        self.expectation == other.expectation && self.saved_state == other.saved_state
    }
}

/// No command replay: an error returned by SQL COMMIT can have an uncertain
/// durable outcome. Resolve that known commit UUID using a fresh safe reader.
#[derive(Debug)]
pub(super) enum CommitOutcome {
    Committed(Box<Snapshot>),
    RejectedBefore(AppError),
    Indeterminate { commit_id: String, error: AppError },
}

pub(super) async fn read(connection: &mut SqliteConnection) -> Result<Snapshot> {
    read::read(connection).await
}

/// Checks project/cursor and the declared immutable commit seals, without
/// exporting events. History bytes remain unverified until an explicit full read.
pub(super) async fn read_metadata(connection: &mut SqliteConnection) -> Result<Snapshot> {
    read::read_scoped(connection, ReadScope::Metadata).await
}

pub(super) async fn read_compatible(
    connection: &mut SqliteConnection,
    original: &Snapshot,
) -> Result<Snapshot> {
    compatibility::read(connection, original).await
}

/// Caller owns an existing transaction (a RESERVED writer transaction for
/// promotion/publication). Never use this as an untransactional reader.
pub(super) async fn read_in_transaction(connection: &mut SqliteConnection) -> Result<Snapshot> {
    read::within(connection).await
}

/// Only creates an internally verified empty database. Caller must configure
/// DELETE/FULL, verify the physical DB and install immutable assets first.
pub(super) async fn initialize(
    connection: &mut SqliteConnection,
    repository_id: &str,
    project: &Project,
    imported_history: Option<&[u8]>,
) -> Result<CommitOutcome> {
    write::initialize(connection, repository_id, project, imported_history).await
}

pub(super) async fn commit(
    connection: &mut SqliteConnection,
    expected: &Snapshot,
    project: &Project,
    transition: Option<&PreparedHistory>,
) -> Result<CommitOutcome> {
    write::commit(connection, expected, project, transition).await
}

fn invalid(code: &str) -> AppError {
    AppError::Invalid(code.into())
}
fn changed() -> AppError {
    invalid("PROJECT_SNAPSHOT_CHANGED")
}
fn unsupported() -> AppError {
    invalid("PROJECT_FOLDER_FORMAT_UNSUPPORTED")
}
fn corrupt() -> AppError {
    invalid("PROJECT_FOLDER_STATE_INVALID")
}
fn limited() -> AppError {
    invalid("PROJECT_FOLDER_LIMIT")
}

fn normalized_project(project: &Project) -> Result<()> {
    project.validate()?;
    if project.files.iter().any(|file| {
        file.data.is_some()
            || file
                .sha256
                .as_deref()
                .is_none_or(|hash| !crate::artifacts::valid_digest(hash))
    }) {
        return Err(corrupt());
    }
    let total = project
        .files
        .iter()
        .try_fold(0_u64, |total, file| total.checked_add(file.size))
        .ok_or_else(limited)?;
    if total > 100 * 1024 * 1024 {
        return Err(limited());
    }
    if serde_json::to_vec(project)?.len() as i64 > MAX_PROJECT_BYTES {
        return Err(limited());
    }
    Ok(())
}

type SchemaObjects = Vec<(String, String, String, Option<String>)>;
/// Compile the frozen DDL once in a separate memory-only database. This also
/// captures engine-defined autoindexes/sequence tables without broad sqlite_*
/// exclusions that could conceal foreign executable schema objects.
async fn expected_schema() -> Result<&'static SchemaObjects> {
    static EXPECTED: tokio::sync::OnceCell<SchemaObjects> = tokio::sync::OnceCell::const_new();
    EXPECTED
        .get_or_try_init(|| async {
            let mut memory = SqliteConnection::connect("sqlite::memory:").await?;
            let ddl = schema_ddl();
            sqlx::Executor::execute(&mut memory, ddl.as_str()).await?;
            let rows =
                sqlx::query("SELECT type,name,tbl_name,sql FROM sqlite_schema ORDER BY name")
                    .fetch_all(&mut memory)
                    .await?;
            let mut expected: SchemaObjects = Vec::with_capacity(rows.len());
            for row in rows {
                expected.push((
                    row.try_get("type")?,
                    row.try_get("name")?,
                    row.try_get("tbl_name")?,
                    row.try_get::<Option<String>, _>("sql")?
                        .map(|sql| sql.trim().to_owned()),
                ));
            }
            memory.close().await?;
            Ok::<_, AppError>(expected)
        })
        .await
}
fn schema_ddl() -> String {
    SCHEMA.replace("\r\n", "\n")
}
fn schema_hash() -> String {
    crate::artifacts::digest(schema_ddl().as_bytes())
}
async fn schema_fingerprint() -> Result<String> {
    Ok(crate::artifacts::digest(&serde_json::to_vec(
        expected_schema().await?,
    )?))
}

struct CommitRecord {
    id: String,
    parent: Option<String>,
    sequence: u64,
    revision: Option<String>,
    project_hash: String,
    cursor_hash: String,
    history_hash: String,
    seal: String,
}
fn record_seal(repository: &str, project: &str, record: &CommitRecord) -> Result<String> {
    Ok(crate::artifacts::digest(&serde_json::to_vec(&(
        repository,
        project,
        &record.id,
        &record.parent,
        record.sequence,
        &record.revision,
        &record.project_hash,
        &record.cursor_hash,
        &record.history_hash,
    ))?))
}

#[cfg(test)]
#[path = "format/tests.rs"]
mod tests;
