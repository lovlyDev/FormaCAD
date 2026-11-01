//! A source owns its writer lease through every VFS base-file close. The weak
//! virtual-name registry never constitutes ownership and never covers IO.
use super::{abi, invalid, platform};
use crate::{
    core::{AppError, Result},
    project_access::folder_lease::FolderLease,
};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU8, Ordering},
        Arc, Mutex, OnceLock, Weak,
    },
};

pub(super) const MAX_DATABASE_BYTES: u64 = 512 * 1024 * 1024;
pub(super) const MAX_JOURNAL_BYTES: u64 = MAX_DATABASE_BYTES * 2;
const PREFIX: &str = "/forma-rw/";
static SOURCES: OnceLock<Mutex<HashMap<String, Weak<Source>>>> = OnceLock::new();
#[cfg(test)]
type BeforeOpen = Box<dyn FnOnce() -> std::result::Result<(), ()> + Send>;

pub(super) struct Source {
    root: PathBuf,
    // Deliberately retained before the first base xOpen and after all base xClose.
    lease: FolderLease,
    pub binding: platform::Binding,
    assets: platform::Binding,
    identity: platform::FileIdentity,
    journal: Mutex<Option<platform::FileIdentity>>,
    promoted: AtomicBool,
    fault: AtomicU8,
    token: String,
    #[cfg(test)]
    pub before_base_open: Mutex<Option<BeforeOpen>>,
}

impl Source {
    pub fn capture(root: &Path, lease: FolderLease) -> Result<Arc<Self>> {
        lease.recheck()?;
        let root = std::path::absolute(root)?;
        let pinned = lease.database_path();
        // The lease's Linux parent is a deliberate /proc/self/fd capability
        // symlink. A trailing '.' makes O_NOFOLLOW apply to the directory,
        // rather than rejecting our own retained FD alias as a leaf symlink.
        let pinned_root = pinned.parent().ok_or_else(invalid)?.join(".");
        let binding = platform::Binding::capture(&pinned_root)?;
        if !binding.matches_directory(&root)? {
            return Err(invalid());
        }
        let record = binding.file_record("project.sqlite")?;
        if !(512..=MAX_DATABASE_BYTES).contains(&record.size) {
            return Err(invalid());
        }
        let identity = record.identity;
        let assets = binding.subdirectory("assets")?;
        let source = Arc::new(Self {
            root,
            lease,
            binding,
            assets,
            identity,
            journal: Mutex::new(None),
            promoted: AtomicBool::new(false),
            fault: AtomicU8::new(0),
            token: uuid::Uuid::new_v4().to_string(),
            #[cfg(test)]
            before_base_open: Mutex::new(None),
        });
        source.check()?;
        let mut sources = SOURCES
            .get_or_init(Default::default)
            .lock()
            .map_err(|_| invalid())?;
        sources.retain(|_, source| source.strong_count() != 0);
        sources.insert(source.token.clone(), Arc::downgrade(&source));
        Ok(source)
    }
    pub fn virtual_name(&self) -> String {
        format!("{PREFIX}{}/project.sqlite", self.token)
    }
    pub fn promoted(&self) -> bool {
        self.promoted.load(Ordering::Acquire)
    }
    pub fn promote(&self) -> Result<()> {
        self.check()?;
        self.promoted.store(true, Ordering::Release);
        Ok(())
    }
    pub fn fail(&self, code: u8) {
        let _ = self
            .fault
            .compare_exchange(0, code, Ordering::AcqRel, Ordering::Acquire);
    }
    pub fn error(&self) -> Option<AppError> {
        match self.fault.load(Ordering::Acquire) {
            0 => None,
            1 => Some(AppError::Invalid(
                "PROJECT_SNAPSHOT_RECOVERY_REQUIRED".into(),
            )),
            _ => Some(invalid()),
        }
    }
    fn journal_id(&self) -> Result<Option<platform::FileIdentity>> {
        Ok(*self.journal.lock().map_err(|_| invalid())?)
    }
    pub fn check(&self) -> Result<()> {
        let result = self.check_current();
        // Once this opening observes loss/replacement it cannot be revived by
        // moving the old pathname back. Preserve an earlier recovery reason.
        if result.is_err() {
            self.fail(2);
        }
        result
    }
    fn check_current(&self) -> Result<()> {
        if let Some(error) = self.error() {
            return Err(error);
        }
        self.lease.recheck()?;
        if !self.binding.matches_directory(&self.root)?
            || !self
                .assets
                .matches_directory(&self.binding.path("assets"))?
            || self.binding.file_record("project.sqlite")?.identity != self.identity
            || platform::path_identity(&self.root.join("project.sqlite"))? != self.identity
        {
            self.fail(2);
            return Err(invalid());
        }
        let size = fs::symlink_metadata(self.binding.path("project.sqlite"))?.len();
        if !(512..=MAX_DATABASE_BYTES).contains(&size) {
            self.fail(2);
            return Err(invalid());
        }
        for suffix in ["-wal", "-shm"] {
            self.absent(&format!("project.sqlite{suffix}"))?;
        }
        match self.journal_id()? {
            Some(identity) if self.promoted() => {
                let journal = self.binding.file_record("project.sqlite-journal")?;
                if journal.identity != identity || journal.size > MAX_JOURNAL_BYTES {
                    self.fail(2);
                    return Err(invalid());
                }
            }
            Some(_) => {
                self.fail(2);
                return Err(invalid());
            }
            None => self.absent("project.sqlite-journal")?,
        }
        self.lease.recheck()
    }
    fn absent(&self, leaf: &str) -> Result<()> {
        match fs::symlink_metadata(self.binding.path(leaf)) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Ok(_) => {
                self.fail(1);
                Err(self.error().unwrap_or_else(invalid))
            }
            Err(error) => Err(error.into()),
        }
    }
    pub fn can_write(&self, journal: bool) -> Result<()> {
        if !self.promoted() {
            return Err(invalid());
        }
        self.check()?;
        // Main data changes require our own exclusive-created rollback journal.
        if !journal && self.journal_id()?.is_none() {
            return Err(invalid());
        }
        Ok(())
    }
    pub fn begin_journal(&self) -> Result<()> {
        if !self.promoted() || self.journal_id()?.is_some() {
            return Err(invalid());
        }
        self.check()
    }
    /// Called only after a CREATE|EXCLUSIVE base open. No existing file is ever
    /// adopted as our journal, and no journal writes precede this attestation.
    pub unsafe fn claim_journal(&self, base: *mut abi::File) -> Result<()> {
        let record = unsafe {
            self.binding
                .actual_file_record(base, "project.sqlite-journal")
        }?;
        if record.size != 0 {
            self.fail(2);
            return Err(invalid());
        }
        let mut journal = self.journal.lock().map_err(|_| invalid())?;
        if journal.is_some() || !self.promoted() {
            return Err(invalid());
        }
        *journal = Some(record.identity);
        drop(journal);
        self.check()
    }
    pub fn journal_owned(&self) -> Result<bool> {
        Ok(self.journal_id()?.is_some())
    }
    pub fn journal_deleted(&self) -> Result<()> {
        *self.journal.lock().map_err(|_| invalid())? = None;
        self.check()
    }
    /// SAFETY: the base file is live and exclusively accessed by xOpen or SQLx.
    pub unsafe fn attest(&self, base: *mut abi::File) -> Result<()> {
        self.check()?;
        if unsafe { self.binding.actual_file_record(base, "project.sqlite") }?.identity
            != self.identity
        {
            self.fail(2);
            return Err(invalid());
        }
        let methods = unsafe { &*(*base).pMethods };
        let mut header = [0_u8; 100];
        let mut size = 0_i64;
        let read = methods.xRead.ok_or_else(invalid)?;
        let file_size = methods.xFileSize.ok_or_else(invalid)?;
        if unsafe { read(base, header.as_mut_ptr().cast(), 100, 0) } != abi::OK
            || unsafe { file_size(base, &mut size) } != abi::OK
            || &header[..16] != b"SQLite format 3\0"
            || !(512..=MAX_DATABASE_BYTES as i64).contains(&size)
        {
            self.fail(2);
            return Err(invalid());
        }
        if header[18] != 1 || header[19] != 1 {
            self.fail(1);
            return Err(self.error().unwrap_or_else(invalid));
        }
        self.check()
    }
}

pub(super) fn lookup(name: &str) -> Option<(Arc<Source>, &str)> {
    let (token, leaf) = name.strip_prefix(PREFIX)?.split_once('/')?;
    if token.len() != 36
        || ![
            "project.sqlite",
            "project.sqlite-journal",
            "project.sqlite-wal",
            "project.sqlite-shm",
        ]
        .contains(&leaf)
    {
        return None;
    }
    let source = SOURCES.get()?.lock().ok()?.get(token)?.upgrade()?;
    Some((source, leaf))
}
