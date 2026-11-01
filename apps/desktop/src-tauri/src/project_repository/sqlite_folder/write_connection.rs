//! Guarded existing-FCP1 writer opening. A non-default VFS owns the source lease
//! before SQLite can access the physical file. Opening is initially unpromoted:
//! reads and a RESERVED lock are allowed, but recovery, journal creation and all
//! mutations are denied until the format and captured saved state are verified.
//! This is not the create-new database path and never installs schema/recovery.
use super::format;
use crate::{
    core::{AppError, Result},
    project_access::folder_lease::FolderLease,
};
use sqlx::{Connection, SqliteConnection};
use std::path::Path;

#[cfg(any(target_os = "linux", windows))]
#[path = "read_only/abi.rs"]
mod abi;
#[cfg(any(target_os = "linux", windows))]
mod authorizer;
#[cfg(any(target_os = "linux", windows))]
mod io;
#[cfg(target_os = "linux")]
#[path = "read_only/linux.rs"]
mod platform;
#[cfg(windows)]
#[path = "read_only/windows.rs"]
mod platform;
#[cfg(any(target_os = "linux", windows))]
mod source;
#[cfg(any(target_os = "linux", windows))]
mod vfs;

fn invalid() -> AppError {
    AppError::Invalid("PROJECT_SNAPSHOT_CHANGED".into())
}
fn unsupported() -> AppError {
    AppError::Invalid("FOLDER_SQLITE_WRITE_UNSUPPORTED".into())
}

pub(super) struct WriteConnection {
    connection: SqliteConnection,
    #[cfg(any(target_os = "linux", windows))]
    source: std::sync::Arc<source::Source>,
}

impl WriteConnection {
    pub(super) async fn open(
        root: &Path,
        lease: FolderLease,
        expected: &format::Snapshot,
    ) -> Result<Self> {
        #[cfg(any(target_os = "linux", windows))]
        {
            Self::open_source(source::Source::capture(root, lease)?, expected).await
        }
        #[cfg(not(any(target_os = "linux", windows)))]
        {
            let _ = (root, lease, expected);
            Err(unsupported())
        }
    }

    #[cfg(any(target_os = "linux", windows))]
    async fn open_source(
        source: std::sync::Arc<source::Source>,
        expected: &format::Snapshot,
    ) -> Result<Self> {
        let mut writer = Self::connect_unpromoted(source).await?;
        // RESERVED excludes another writer while schema/seals are validated.
        // The unpromoted VFS still refuses EXCLUSIVE/recovery/journal/writes.
        let mut transaction = writer.connection.begin_with("BEGIN IMMEDIATE").await?;
        let current = format::read_in_transaction(&mut transaction).await?;
        if !expected.same_saved_state(&current) {
            return Err(invalid());
        }
        writer.source.check()?;
        writer.source.promote()?;
        transaction.rollback().await?;
        writer.attest().await?;
        Ok(writer)
    }

    #[cfg(any(target_os = "linux", windows))]
    async fn connect_unpromoted(source: std::sync::Arc<source::Source>) -> Result<Self> {
        if !vfs::register() {
            return Err(unsupported());
        }
        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(source.virtual_name())
            .vfs(vfs::NAME)
            .read_only(false)
            .create_if_missing(false)
            .busy_timeout(std::time::Duration::ZERO)
            .statement_cache_capacity(0)
            .optimize_on_close(false, None)
            // These are connection-local and must not mutate the file before
            // validation. In particular do not request journal_mode here.
            .pragma("foreign_keys", "ON")
            .pragma("temp_store", "MEMORY")
            .pragma("trusted_schema", "OFF")
            .pragma("synchronous", "FULL");
        let connection = SqliteConnection::connect_with(&options)
            .await
            .map_err(|error| source.error().unwrap_or_else(|| error.into()))?;
        let mut writer = Self { connection, source };
        writer.attest().await?;
        {
            let mut handle = writer.connection.lock_handle().await?;
            // SAFETY: the connection worker is parked and the callback is static.
            let rc = unsafe {
                abi::sqlite3_set_authorizer(
                    handle.as_raw_handle().as_ptr().cast(),
                    Some(authorizer::authorize),
                    std::ptr::null_mut(),
                )
            };
            if rc != abi::OK {
                return Err(invalid());
            }
        }
        Ok(writer)
    }

    /// Trusted repository SQL only. The caller must compare a captured Snapshot
    /// inside the same RESERVED transaction used for publication, never merely
    /// recheck before entering that transaction.
    pub(super) fn conn(&mut self) -> &mut SqliteConnection {
        &mut self.connection
    }

    pub(super) async fn recheck(&mut self, expected: &format::Snapshot) -> Result<()> {
        self.attest().await?;
        let current = format::read(&mut self.connection).await?;
        if !expected.same_saved_state(&current) {
            return Err(invalid());
        }
        self.attest().await
    }

    async fn attest(&mut self) -> Result<()> {
        #[cfg(any(target_os = "linux", windows))]
        {
            self.source.check()?;
            let rc = {
                let mut handle = self.connection.lock_handle().await?;
                // SAFETY: VFS ATTEST accesses this connection's actual opened
                // main file while SQLx holds exclusive handle access.
                unsafe {
                    abi::sqlite3_file_control(
                        handle.as_raw_handle().as_ptr().cast(),
                        c"main".as_ptr(),
                        abi::ATTEST,
                        std::ptr::null_mut(),
                    )
                }
            };
            if rc != abi::OK {
                return Err(self.source.error().unwrap_or_else(invalid));
            }
            self.source.check()
        }
        #[cfg(not(any(target_os = "linux", windows)))]
        {
            Err(unsupported())
        }
    }

    pub(super) async fn close(self) -> Result<()> {
        self.connection.close().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
