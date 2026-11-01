//! Guarded existing-FCP1 writer opening. A non-default VFS owns the source lease
//! before SQLite can access the physical file. Opening is initially unpromoted:
//! reads and a RESERVED lock are allowed, but recovery, journal creation and all
//! mutations are denied until the format and captured saved state are verified.
//! Creation has separate private authority for format::initialize only; existing
//! files can never enter that mode. Neither mode performs recovery.
use super::format;
use crate::{
    core::{AppError, Result},
    project_access::folder_lease::FolderLease,
};
use sqlx::{Connection, SqliteConnection};
use std::path::Path;

#[cfg(any(target_os = "linux", windows))]
use super::read_only::{abi, platform};
#[cfg(any(target_os = "linux", windows))]
mod authorizer;
#[cfg(any(target_os = "linux", windows))]
mod io;
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
    /// Create only after the caller validated the project/history, obtained an
    /// initialize_empty lease and durably published immutable assets. The assets
    /// directory is retained through the actual SQLite base-file close.
    /// Any non-Committed outcome requires discarding this connection; resolve an
    /// indeterminate UUID using a fresh safe reader after closing, never replay.
    pub(super) async fn create(
        root: &Path,
        lease: FolderLease,
        repository_id: &str,
        project: &crate::models::Project,
        history: Option<&[u8]>,
        published: std::sync::Arc<super::cas::PublishedCasBatch>,
    ) -> Result<(Self, format::CommitOutcome)> {
        #[cfg(any(target_os = "linux", windows))]
        {
            let source = source::Source::capture_new(root, lease, published)?;
            let mut writer = Self::connect_unpromoted(source).await?;
            // A fresh SQLite file defaults to DELETE; do not run a mode-switch
            // pragma which could acquire unrelated authority or hide recovery.
            let journal: String = sqlx::query_scalar("PRAGMA journal_mode")
                .fetch_one(&mut writer.connection)
                .await?;
            if !journal.eq_ignore_ascii_case("delete") {
                return Err(invalid());
            }
            let outcome =
                format::initialize(&mut writer.connection, repository_id, project, history).await?;
            let outcome = match outcome {
                format::CommitOutcome::Committed(snapshot) => {
                    let commit_id = snapshot.expectation().commit_id().to_owned();
                    let checked = async {
                        let actual = format::read(&mut writer.connection).await?;
                        if !snapshot.same_saved_state(&actual) {
                            return Err(invalid());
                        }
                        writer.source.finish_creation()?;
                        writer.attest().await?;
                        writer.source.flush_namespace()
                    }
                    .await;
                    match checked {
                        Ok(()) => format::CommitOutcome::Committed(snapshot),
                        Err(error) => format::CommitOutcome::Indeterminate { commit_id, error },
                    }
                }
                other => other,
            };
            if !matches!(&outcome, format::CommitOutcome::Committed(_)) {
                // SQLx COMMIT errors can poison transaction bookkeeping. Fail
                // closed and retain the source/lease until actual worker close.
                writer.source.fail(2);
            }
            Ok((writer, outcome))
        }
        #[cfg(not(any(target_os = "linux", windows)))]
        {
            let _ = (root, lease, repository_id, project, history, published);
            Err(unsupported())
        }
    }

    pub(super) async fn open(
        root: &Path,
        lease: FolderLease,
        expected: &format::Snapshot,
        binding: super::session::ReadBinding,
    ) -> Result<Self> {
        #[cfg(any(target_os = "linux", windows))]
        {
            Self::open_source(source::Source::capture(root, lease, binding)?, expected).await
        }
        #[cfg(not(any(target_os = "linux", windows)))]
        {
            let _ = (root, lease, expected, binding);
            Err(unsupported())
        }
    }

    /// Retain the published CAS batch before SQLx can start its worker.
    pub(super) async fn open_for_publication(
        root: &Path,
        lease: FolderLease,
        expected: &format::Snapshot,
        binding: super::session::ReadBinding,
        published: std::sync::Arc<super::cas::PublishedCasBatch>,
    ) -> Result<Self> {
        #[cfg(any(target_os = "linux", windows))]
        {
            Self::open_source(
                source::Source::capture_for_publication(root, lease, binding, published)?,
                expected,
            )
            .await
        }
        #[cfg(not(any(target_os = "linux", windows)))]
        {
            let _ = (root, lease, expected, binding, published);
            Err(unsupported())
        }
    }
    #[cfg(any(target_os = "linux", windows))]
    fn open_source(
        source: std::sync::Arc<source::Source>,
        expected: &format::Snapshot,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self>> + Send>> {
        // Own the expected seals before suspending. A type-erased Send boundary
        // keeps SQLx executor lifetimes local and enforces host-task usability.
        let expected = expected.clone();
        Box::pin(async move {
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
        })
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
            .create_if_missing(source.creating())
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
                    std::sync::Arc::as_ptr(&writer.source).cast_mut().cast(),
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
