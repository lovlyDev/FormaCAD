//! Isolated prototype used only by the representation benchmark.
//! It has no app index, grants, agent sessions or application migrations.
//! DELETE rollback journal/FULL is deliberately distinct from the earlier app
//! WAL comparator. Readers use SQLite read-only mode, never immutable mode.
use crate::core::{AppError, Result};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
    SqlitePool,
};
use std::{io::Read, path::Path};

const APPLICATION_ID: i64 = 0x4643_4231;
const USER_VERSION: i64 = 1;
// Exact projects/history columns and constraints from the corresponding app
// migrations, intentionally excluding every app-only table and migration runner.
const SCHEMA: &str = r#"
CREATE TABLE projects (id TEXT PRIMARY KEY NOT NULL, name TEXT NOT NULL, payload TEXT NOT NULL, updated_at TEXT NOT NULL);
CREATE TABLE project_history (
    project_id TEXT PRIMARY KEY NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    state_json TEXT NOT NULL
);
CREATE TABLE project_history_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK(kind IN ('edit', 'undo', 'redo')),
    from_revision TEXT,
    to_revision TEXT,
    target_revision TEXT,
    created_at TEXT NOT NULL
);
CREATE INDEX project_history_events_project ON project_history_events(project_id, id);
"#;

fn invalid() -> AppError {
    AppError::Invalid("Unsupported benchmark SQLite candidate format".into())
}

fn preflight(path: &Path) -> Result<()> {
    // Header layout: https://www.sqlite.org/fileformat.html#the_database_header
    // Refuse static WAL headers before SQLite opens the database: a read-only
    // connection to a WAL database can otherwise create -wal/-shm sidecars.
    // This bounded check is for controlled benchmark fixtures, not a source
    // identity or synchronization mechanism against an external path writer.
    let mut header = [0u8; 100];
    std::fs::File::open(path)?.read_exact(&mut header)?;
    if &header[..16] != b"SQLite format 3\0"
        || header[18] != 1
        || header[19] != 1
        || i64::from(u32::from_be_bytes(header[60..64].try_into().unwrap())) != USER_VERSION
        || i64::from(u32::from_be_bytes(header[68..72].try_into().unwrap())) != APPLICATION_ID
    {
        return Err(invalid());
    }
    Ok(())
}

async fn check_format(pool: &SqlitePool) -> Result<()> {
    let application_id: i64 = sqlx::query_scalar("PRAGMA application_id")
        .fetch_one(pool)
        .await?;
    let version: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(pool)
        .await?;
    if application_id != APPLICATION_ID || version != USER_VERSION {
        return Err(invalid());
    }
    let integrity: String = sqlx::query_scalar("PRAGMA quick_check")
        .fetch_one(pool)
        .await?;
    if integrity != "ok" {
        return Err(invalid());
    }
    Ok(())
}

pub(super) async fn create(path: &Path) -> Result<SqlitePool> {
    // Initialization is explicit and never reuses or migrates an existing DB.
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(false)
        .journal_mode(SqliteJournalMode::Delete)
        .synchronous(SqliteSynchronous::Full)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;
    let mut transaction = pool.begin().await?;
    sqlx::raw_sql(SCHEMA).execute(&mut *transaction).await?;
    sqlx::query(&format!("PRAGMA application_id = {APPLICATION_ID}"))
        .execute(&mut *transaction)
        .await?;
    sqlx::query(&format!("PRAGMA user_version = {USER_VERSION}"))
        .execute(&mut *transaction)
        .await?;
    transaction.commit().await?;
    check_format(&pool).await?;
    Ok(pool)
}

pub(super) async fn open_reader(path: &Path) -> Result<SqlitePool> {
    // A closed DELETE-mode fixture needs no sidecars. This is only benchmark
    // preflight, not a concurrency protocol: SQLite still supplies read locks.
    for suffix in ["-wal", "-shm", "-journal"] {
        let mut sidecar = path.as_os_str().to_os_string();
        sidecar.push(suffix);
        if Path::new(&sidecar).exists() {
            return Err(invalid());
        }
    }
    preflight(path)?;
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(false)
        .read_only(true)
        .foreign_keys(true)
        .pragma("query_only", "ON");
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;
    if let Err(error) = check_format(&pool).await {
        pool.close().await;
        return Err(error);
    }
    Ok(pool)
}

pub(super) async fn open_writer(path: &Path) -> Result<SqlitePool> {
    // Reject unsupported formats through a pure reader before writer startup.
    let reader = open_reader(path).await?;
    reader.close().await;
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(false)
        .synchronous(SqliteSynchronous::Full)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;
    if let Err(error) = check_format(&pool).await {
        pool.close().await;
        return Err(error);
    }
    sqlx::query("PRAGMA journal_mode = DELETE")
        .execute(&pool)
        .await?;
    Ok(pool)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn files(root: &Path) -> BTreeMap<String, Vec<u8>> {
        std::fs::read_dir(root)
            .unwrap()
            .map(|entry| {
                let entry = entry.unwrap();
                (
                    entry.file_name().to_string_lossy().into_owned(),
                    std::fs::read(entry.path()).unwrap(),
                )
            })
            .collect()
    }

    #[tokio::test]
    async fn reader_uses_only_candidate_schema_and_preserves_every_file() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("project.sqlite");
        let writer = create(&path).await.unwrap();
        writer.close().await;
        let before = files(root.path());
        let reader = open_reader(&path).await.unwrap();
        let names: Vec<String> =
            sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
                .fetch_all(&reader)
                .await
                .unwrap();
        assert_eq!(
            names,
            [
                "project_history",
                "project_history_events",
                "projects",
                "sqlite_sequence"
            ]
        );
        assert!(sqlx::query("DELETE FROM projects")
            .execute(&reader)
            .await
            .is_err());
        reader.close().await;
        assert_eq!(files(root.path()), before);
        root.close().unwrap();
    }

    #[tokio::test]
    async fn future_candidate_version_is_refused_without_creating_or_changing_files() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("project.sqlite");
        let writer = create(&path).await.unwrap();
        sqlx::query("PRAGMA user_version = 2")
            .execute(&writer)
            .await
            .unwrap();
        writer.close().await;
        let before = files(root.path());
        assert!(open_reader(&path).await.is_err());
        assert!(open_writer(&path).await.is_err());
        assert_eq!(files(root.path()), before);
        root.close().unwrap();
    }

    #[tokio::test]
    async fn foreign_format_and_wal_sidecars_are_refused_without_changes() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("project.sqlite");
        let writer = create(&path).await.unwrap();
        sqlx::query("PRAGMA application_id = 0")
            .execute(&writer)
            .await
            .unwrap();
        writer.close().await;
        let before = files(root.path());
        assert!(open_reader(&path).await.is_err());
        assert_eq!(files(root.path()), before);
        root.close().unwrap();
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("project.sqlite");
        let writer = create(&path).await.unwrap();
        writer.close().await;
        // With valid format markers, the reader must not ignore a potentially
        // pending WAL. Test only our synthetic sidecar, never application data.
        std::fs::write(
            root.path().join("project.sqlite-wal"),
            b"synthetic pending WAL",
        )
        .unwrap();
        let with_sidecar = files(root.path());
        assert!(open_reader(&path).await.is_err());
        assert_eq!(files(root.path()), with_sidecar);
        root.close().unwrap();
    }

    #[tokio::test]
    async fn wal_database_is_refused_before_sqlite_can_create_reader_sidecars() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("project.sqlite");
        let writer = create(&path).await.unwrap();
        let mode: String = sqlx::query_scalar("PRAGMA journal_mode = WAL")
            .fetch_one(&writer)
            .await
            .unwrap();
        assert_eq!(mode, "wal");
        writer.close().await;
        let before = files(root.path());
        assert!(open_reader(&path).await.is_err());
        assert_eq!(files(root.path()), before);
        root.close().unwrap();
    }
}
