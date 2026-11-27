//! Read and attest a database/journal pair without opening its SQLite pager.
use super::{abi, blob, invalid, io, platform::FileRecord, source::Source, vfs};
use crate::{
    core::{AppError, Result},
    project_access::folder_lease::FolderLease,
};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
    sync::Arc,
};

const DATABASE: &str = "project.sqlite";
const JOURNAL: &str = "project.sqlite-journal";
const MAX_DATABASE: u64 = super::source::MAX_DATABASE_BYTES;
const MAX_JOURNAL: u64 = MAX_DATABASE * 2;
const JOURNAL_MAGIC: [u8; 8] = [0xd9, 0xd5, 0x05, 0xf9, 0x20, 0xa1, 0x63, 0xd7];

#[derive(PartialEq, Eq)]
pub(in crate::project_repository::sqlite_folder) struct Fingerprint {
    database: StampedFile,
    journal: StampedFile,
}

#[derive(PartialEq, Eq)]
struct StampedFile {
    record: FileRecord,
    sha256: [u8; 32],
}

pub(in crate::project_repository::sqlite_folder) struct RecoveryFiles {
    lease: FolderLease,
    source: Arc<Source>,
}

impl RecoveryFiles {
    pub(in crate::project_repository::sqlite_folder) fn open(root: &Path) -> Result<Self> {
        let lease = FolderLease::acquire(root)?;
        let source = Source::bind(root)?;
        let files = Self { lease, source };
        files.check()?;
        Ok(files)
    }

    pub(in crate::project_repository::sqlite_folder) fn stage(
        &self,
        path: &Path,
    ) -> Result<Fingerprint> {
        self.capture(Some(path))
    }

    pub(in crate::project_repository::sqlite_folder) fn recheck(
        &self,
        expected: &Fingerprint,
    ) -> Result<()> {
        let actual = self.capture(None).map_err(|error| match error {
            AppError::Io(error) if error.kind() == ErrorKind::NotFound => invalid(),
            other => other,
        })?;
        if actual != *expected {
            return Err(invalid());
        }
        Ok(())
    }

    pub(in crate::project_repository::sqlite_folder) fn read_blob(
        &self,
        hash: &str,
        size: u64,
    ) -> Result<Vec<u8>> {
        blob::read_checked(&self.source, hash, size, || self.check())
    }

    fn check(&self) -> Result<()> {
        self.lease.recheck()?;
        self.source.check_binding()?;
        for name in ["project.sqlite-wal", "project.sqlite-shm"] {
            match fs::symlink_metadata(self.source.binding.path(name)) {
                Err(error) if error.kind() == ErrorKind::NotFound => (),
                Err(error) => return Err(error.into()),
                Ok(_) => return Err(unsupported()),
            }
        }
        Ok(())
    }

    fn capture(&self, scratch: Option<&Path>) -> Result<Fingerprint> {
        self.check()?;
        let binding = &self.source.binding;
        let main = self.open_file(DATABASE, 512..=MAX_DATABASE)?;
        // SAFETY: this base handle is uniquely held until the complete capture
        // finishes. xClose from the audited VFS releases its SHARED lock.
        let lock = unsafe { &*(*main.handle.base).pMethods }
            .xLock
            .ok_or_else(invalid)?;
        if unsafe { lock(main.handle.base, 1) } != abi::OK {
            return Err(invalid());
        }
        main.require_no_writer()?;
        let journal = self.open_file(JOURNAL, 0..=MAX_JOURNAL)?;
        let database_stamp = main.copy_to(scratch.map(|path| path.join(DATABASE)))?;
        let journal_stamp = journal.copy_to(scratch.map(|path| path.join(JOURNAL)))?;
        self.check()?;
        main.require_no_writer()?;
        for file in [&main, &journal] {
            if unsafe { binding.actual_file_record(file.handle.base, file.name) }? != file.record {
                return Err(invalid());
            }
        }
        Ok(Fingerprint {
            database: database_stamp,
            journal: journal_stamp,
        })
    }

    fn open_file(
        &self,
        name: &'static str,
        range: std::ops::RangeInclusive<u64>,
    ) -> Result<BoundFile> {
        let binding = &self.source.binding;
        let record = binding.file_record(name)?;
        if !range.contains(&record.size) {
            return Err(unsupported());
        }
        let handle = vfs::data_file(Arc::clone(&self.source), &binding.alias(name)?)?;
        if unsafe { binding.actual_file_record(handle.base, name) }? != record {
            return Err(invalid());
        }
        Ok(BoundFile {
            handle,
            name,
            record,
        })
    }
}

struct BoundFile {
    handle: Box<io::State>,
    name: &'static str,
    record: FileRecord,
}

impl BoundFile {
    fn require_no_writer(&self) -> Result<()> {
        let methods = unsafe { &*(*self.handle.base).pMethods };
        let check = methods.xCheckReservedLock.ok_or_else(invalid)?;
        let mut reserved = 0;
        if unsafe { check(self.handle.base, &mut reserved) } != abi::OK || reserved != 0 {
            return Err(invalid());
        }
        Ok(())
    }

    fn copy_to(&self, output: Option<PathBuf>) -> Result<StampedFile> {
        let methods = unsafe { &*(*self.handle.base).pMethods };
        let read = methods.xRead.ok_or_else(invalid)?;
        let mut output = output.map(File::create_new).transpose()?;
        let mut sha = Sha256::new();
        let mut buffer = vec![0_u8; 64 * 1024];
        let mut position = 0;
        let mut tail = [0_u8; JOURNAL_MAGIC.len()];
        while position < self.record.size {
            let size = (self.record.size - position).min(buffer.len() as u64) as usize;
            let chunk = &mut buffer[..size];
            // SAFETY: xRead receives a live buffer and bounded offset. The
            // pinned handle stays owned across the complete read.
            if unsafe {
                read(
                    self.handle.base,
                    chunk.as_mut_ptr().cast(),
                    size as i32,
                    position as i64,
                )
            } != abi::OK
            {
                return Err(invalid());
            }
            if let Some(file) = &mut output {
                file.write_all(chunk)?;
            }
            let retained = size.min(tail.len());
            tail.rotate_left(retained);
            let start = tail.len() - retained;
            tail[start..].copy_from_slice(&chunk[size - retained..]);
            sha.update(chunk);
            position += size as u64;
        }
        // SQLite 3.46 recognizes a super-journal pathname only with this
        // trailer. FCP1 never attaches databases. Do not pass this journal
        // to SQLite, even in a private scratch directory.
        if self.name == JOURNAL && tail == JOURNAL_MAGIC {
            return Err(unsupported());
        }
        Ok(StampedFile {
            record: self.record,
            sha256: sha.finalize().into(),
        })
    }
}

fn unsupported() -> AppError {
    AppError::Invalid("PROJECT_SNAPSHOT_RECOVERY_REQUIRED".into())
}
