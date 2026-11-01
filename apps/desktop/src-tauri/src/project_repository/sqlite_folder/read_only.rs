//! Pure live-SQLite reads through a narrow, non-default IO policy.
//!
//! No schema migration, recovery, writer lease or raw OS database descriptor is
//! opened here. Linux uses a retained directory alias and SQLite's own inode
//! bookkeeping. Windows checks the actual borrowed SQLite HANDLE. Other
//! platforms fail closed; no macOS/NAS guarantee is implied.
use crate::core::{AppError, Result};
#[cfg(any(target_os = "linux", windows))]
use sqlx::{Connection, SqliteConnection};
use std::path::Path;

#[cfg(any(target_os = "linux", windows))]
#[path = "read_only/abi.rs"]
mod abi;
#[cfg(any(target_os = "linux", windows))]
#[path = "read_only/authorizer.rs"]
mod authorizer;
#[cfg(any(target_os = "linux", windows))]
#[path = "read_only/blob.rs"]
mod blob;
#[cfg(any(target_os = "linux", windows))]
#[path = "read_only/io.rs"]
mod io;
#[cfg(target_os = "linux")]
#[path = "read_only/linux.rs"]
mod platform;
#[cfg(windows)]
#[path = "read_only/windows.rs"]
mod platform;
#[cfg(any(target_os = "linux", windows))]
#[path = "read_only/source.rs"]
mod source;
#[cfg(any(target_os = "linux", windows))]
#[path = "read_only/vfs.rs"]
mod vfs;

#[cfg(any(target_os = "linux", windows))]
fn invalid() -> AppError {
    AppError::Invalid("PROJECT_SNAPSHOT_CHANGED".into())
}
fn unsupported() -> AppError {
    AppError::Invalid("FOLDER_SQLITE_READ_UNSUPPORTED".into())
}

/// Retains the actual SQLite file for the entire opening. After every awaited
/// query and before accepting a result the caller must recheck this opening and
/// its own captured commit/revision seals. This is not a writer capability.
pub(super) struct ReadOnlyDatabase {
    #[cfg(any(target_os = "linux", windows))]
    connection: SqliteConnection,
    #[cfg(any(target_os = "linux", windows))]
    source: std::sync::Arc<source::Source>,
}

impl ReadOnlyDatabase {
    pub(super) async fn open(root: &Path) -> Result<Self> {
        #[cfg(any(target_os = "linux", windows))]
        {
            Self::open_source(source::Source::capture(root)?).await
        }
        #[cfg(not(any(target_os = "linux", windows)))]
        {
            let _ = root;
            Err(unsupported())
        }
    }

    #[cfg(any(target_os = "linux", windows))]
    async fn open_source(source: std::sync::Arc<source::Source>) -> Result<Self> {
        if !vfs::register() {
            return Err(unsupported());
        }
        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(source.virtual_name())
            .vfs(vfs::NAME)
            .read_only(true)
            .create_if_missing(false)
            .busy_timeout(std::time::Duration::ZERO)
            .statement_cache_capacity(0)
            .optimize_on_close(false, None)
            .pragma("query_only", "ON")
            .pragma("temp_store", "MEMORY")
            .pragma("trusted_schema", "OFF");
        let mut connection = SqliteConnection::connect_with(&options)
            .await
            .map_err(|error| source.error().unwrap_or_else(|| error.into()))?;
        source.check()?;
        {
            let mut handle = connection.lock_handle().await?;
            // SAFETY: SQLx parks its worker and holds exclusive access to this
            // connection. Static callback/context remain valid until close.
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
        let mut database = Self { connection, source };
        database.recheck().await?;
        Ok(database)
    }

    /// Private repository query access. Writes/ATTACH are denied independently
    /// by SQL policy and by VFS policy; callers still bound rows/payload sizes.
    #[cfg(any(target_os = "linux", windows))]
    pub(super) fn conn(&mut self) -> &mut SqliteConnection {
        &mut self.connection
    }

    /// Bounded CAS read through SQLite's own OS-file bookkeeping. This avoids
    /// raw FD-close lock loss even if a malicious CAS leaf aliases the DB.
    /// The caller must subsequently recheck its SQL commit/revision seals.
    #[cfg(any(target_os = "linux", windows))]
    pub(super) fn read_blob(&mut self, hash: &str, size: u64) -> Result<Vec<u8>> {
        #[cfg(any(target_os = "linux", windows))]
        {
            blob::read(&self.source, hash, size)
        }
        #[cfg(not(any(target_os = "linux", windows)))]
        {
            let _ = (hash, size);
            Err(unsupported())
        }
    }

    #[cfg(any(target_os = "linux", windows))]
    pub(super) async fn recheck(&mut self) -> Result<()> {
        #[cfg(any(target_os = "linux", windows))]
        {
            self.source.check()?;
            let rc = {
                let mut handle = self.connection.lock_handle().await?;
                // SAFETY: exclusive worker access; main is the original VFS
                // file, and ATTEST is our private non-mutating file control.
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

    #[cfg(any(target_os = "linux", windows))]
    pub(super) async fn close(self) -> Result<()> {
        self.connection.close().await?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "read_only/tests.rs"]
mod tests;
