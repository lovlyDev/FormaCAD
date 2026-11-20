//! Disposable application metadata. Entries never authorize opening a folder.
use crate::core::{AppError, Result};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteConnection},
    Connection, Sqlite, Transaction,
};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

const APPLICATION_ID: i64 = 0x46434931; // FCI1
const FORMAT_VERSION: i64 = 1;

pub struct FolderIndex {
    path: PathBuf,
}

#[derive(Debug, PartialEq, Eq)]
pub struct RecentFolder {
    pub location: PathBuf,
    pub last_opened_at_ms: i64,
    pub pinned: bool,
}

impl FolderIndex {
    pub fn in_app_data(root: &Path) -> Self {
        Self {
            path: root.join("folder-locations.sqlite"),
        }
    }

    /// Record only a location confirmed by an explicit, successful host opening.
    pub async fn record_open(&self, location: &Path) -> Result<()> {
        let location = location_key(location)?;
        let mut db = self.connect(false).await?;
        let mut transaction = db.begin_with("BEGIN IMMEDIATE").await?;
        prepare_index(&mut transaction).await?;
        sqlx::query(
            "INSERT INTO recent_folders(location, last_opened_at_ms, pinned)
             SELECT ?1, ?2, 0 FROM index_settings WHERE id = 1 AND remember_recent = 1
             ON CONFLICT(location) DO UPDATE SET last_opened_at_ms = excluded.last_opened_at_ms",
        )
        .bind(location)
        .bind(chrono::Utc::now().timestamp_millis())
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(())
    }

    pub async fn recent(&self) -> Result<Vec<RecentFolder>> {
        if !self.path.try_exists()? {
            return Ok(Vec::new());
        }
        let mut db = self.connect(true).await?;
        let mut transaction = db.begin().await?;
        verify_index(&mut transaction).await?;
        let rows = sqlx::query_as::<_, (String, i64, bool)>(
            "SELECT location, last_opened_at_ms, pinned FROM recent_folders
             ORDER BY pinned DESC, last_opened_at_ms DESC, location",
        )
        .fetch_all(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(rows
            .into_iter()
            .map(|(location, last_opened_at_ms, pinned)| RecentFolder {
                location: location.into(),
                last_opened_at_ms,
                pinned,
            })
            .collect())
    }

    pub async fn set_pinned(&self, location: &Path, pinned: bool) -> Result<bool> {
        let location = location_key(location)?;
        let mut db = self.connect(false).await?;
        let mut transaction = db.begin_with("BEGIN IMMEDIATE").await?;
        prepare_index(&mut transaction).await?;
        let result = sqlx::query("UPDATE recent_folders SET pinned = ?1 WHERE location = ?2")
            .bind(pinned)
            .bind(location)
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(result.rows_affected() == 1)
    }

    pub async fn remember_recent(&self) -> Result<bool> {
        if !self.path.try_exists()? {
            return Ok(true);
        }
        let mut db = self.connect(true).await?;
        let mut transaction = db.begin().await?;
        verify_index(&mut transaction).await?;
        let remember =
            sqlx::query_scalar("SELECT remember_recent FROM index_settings WHERE id = 1")
                .fetch_one(&mut *transaction)
                .await?;
        transaction.commit().await?;
        Ok(remember)
    }

    pub async fn set_remember_recent(&self, enabled: bool) -> Result<()> {
        let mut db = self.connect(false).await?;
        let mut transaction = db.begin_with("BEGIN IMMEDIATE").await?;
        prepare_index(&mut transaction).await?;
        sqlx::query("UPDATE index_settings SET remember_recent = ?1 WHERE id = 1")
            .bind(enabled)
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(())
    }

    async fn connect(&self, read_only: bool) -> Result<SqliteConnection> {
        let options = SqliteConnectOptions::new()
            .filename(&self.path)
            .read_only(read_only)
            .create_if_missing(!read_only)
            .busy_timeout(Duration::from_secs(2));
        Ok(SqliteConnection::connect_with(&options).await?)
    }
}

fn location_key(location: &Path) -> Result<&str> {
    location
        .to_str()
        .filter(|_| location.is_absolute())
        .ok_or_else(|| AppError::Invalid("PROJECT_FOLDER_INDEX_LOCATION_INVALID".into()))
}

async fn verify_index(db: &mut Transaction<'_, Sqlite>) -> Result<()> {
    if index_identity(db).await? == (APPLICATION_ID, FORMAT_VERSION) {
        return Ok(());
    }
    Err(unsupported())
}

async fn prepare_index(db: &mut Transaction<'_, Sqlite>) -> Result<()> {
    match index_identity(db).await? {
        (APPLICATION_ID, FORMAT_VERSION) => return Ok(()),
        (0, 0) => {}
        _ => return Err(unsupported()),
    }
    let schema_empty: bool = sqlx::query_scalar("SELECT NOT EXISTS (SELECT 1 FROM sqlite_schema)")
        .fetch_one(&mut **db)
        .await?;
    if !schema_empty {
        return Err(unsupported());
    }
    sqlx::raw_sql(
        "CREATE TABLE recent_folders (
             location TEXT PRIMARY KEY NOT NULL,
             last_opened_at_ms INTEGER NOT NULL,
             pinned INTEGER NOT NULL CHECK(pinned IN (0, 1))
         );
         CREATE TABLE index_settings (
             id INTEGER PRIMARY KEY CHECK(id = 1),
             remember_recent INTEGER NOT NULL CHECK(remember_recent IN (0, 1))
         );
         INSERT INTO index_settings VALUES (1, 1);",
    )
    .execute(&mut **db)
    .await?;
    sqlx::query(&format!("PRAGMA application_id = {APPLICATION_ID}"))
        .execute(&mut **db)
        .await?;
    sqlx::query(&format!("PRAGMA user_version = {FORMAT_VERSION}"))
        .execute(&mut **db)
        .await?;
    Ok(())
}

async fn index_identity(db: &mut Transaction<'_, Sqlite>) -> Result<(i64, i64)> {
    let app = sqlx::query_scalar("PRAGMA application_id")
        .fetch_one(&mut **db)
        .await?;
    let version = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(&mut **db)
        .await?;
    Ok((app, version))
}

fn unsupported() -> AppError {
    AppError::Invalid("PROJECT_FOLDER_INDEX_FORMAT_UNSUPPORTED".into())
}

#[cfg(test)]
mod tests;
