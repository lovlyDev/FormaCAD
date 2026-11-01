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
    original: Option<super::super::session::ReadBinding>,
    published: Option<Arc<super::super::cas::PublishedCasBatch>>,
    // Deliberately retained before the first base xOpen and after all base xClose.
    lease: FolderLease,
    pub binding: platform::Binding,
    assets: platform::Binding,
    identity: Mutex<Option<platform::FileIdentity>>,
    creating: AtomicBool,
    main_open_started: AtomicBool,
    journal: Mutex<Option<platform::FileIdentity>>,
    promoted: AtomicBool,
    fault: AtomicU8,
    token: String,
    #[cfg(test)]
    pub before_base_open: Mutex<Option<BeforeOpen>>,
    #[cfg(test)]
    pub after_base_open: Mutex<Option<BeforeOpen>>,
}

impl Source {
    pub fn capture(
        root: &Path,
        lease: FolderLease,
        binding: super::super::session::ReadBinding,
    ) -> Result<Arc<Self>> {
        binding.check_root(root)?;
        Self::capture_mode(root, lease, false, Some(binding), None)
    }
    pub fn capture_new(
        root: &Path,
        lease: FolderLease,
        published: Arc<super::super::cas::PublishedCasBatch>,
    ) -> Result<Arc<Self>> {
        published.recheck_new()?;
        Self::capture_mode(root, lease, true, None, Some(published))
    }
    fn capture_mode(
        root: &Path,
        lease: FolderLease,
        creating: bool,
        original: Option<super::super::session::ReadBinding>,
        published: Option<Arc<super::super::cas::PublishedCasBatch>>,
    ) -> Result<Arc<Self>> {
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
        let identity = if creating {
            match fs::symlink_metadata(binding.path("project.sqlite")) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                _ => return Err(invalid()),
            }
        } else {
            let record = binding.file_record("project.sqlite")?;
            if !(512..=MAX_DATABASE_BYTES).contains(&record.size) {
                return Err(invalid());
            }
            Some(record.identity)
        };
        let assets = binding.subdirectory("assets")?;
        let source = Arc::new(Self {
            root,
            original,
            published,
            lease,
            binding,
            assets,
            identity: Mutex::new(identity),
            creating: AtomicBool::new(creating),
            main_open_started: AtomicBool::new(false),
            journal: Mutex::new(None),
            promoted: AtomicBool::new(false),
            fault: AtomicU8::new(0),
            token: uuid::Uuid::new_v4().to_string(),
            #[cfg(test)]
            before_base_open: Mutex::new(None),
            #[cfg(test)]
            after_base_open: Mutex::new(None),
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
        self.promoted.load(Ordering::Acquire) || self.creating()
    }
    pub fn creating(&self) -> bool {
        self.creating.load(Ordering::Acquire)
    }
    pub fn check_active(&self) -> Result<()> {
        if let Some(original) = &self.original {
            original.check_active()?;
        }
        Ok(())
    }
    pub unsafe fn attest_original(&self, base: *mut abi::File) -> Result<()> {
        if let Some(original) = &self.original {
            original.check_root(&self.root)?;
            unsafe { original.attest_base(base.cast()) }?;
        }
        Ok(())
    }
    pub fn start_main_open(&self) -> Result<()> {
        self.check_active()?;
        if self.main_open_started.swap(true, Ordering::AcqRel) {
            return Err(invalid());
        }
        self.check()
    }
    pub unsafe fn claim_main(&self, base: *mut abi::File) -> Result<()> {
        if !self.creating() {
            return Err(invalid());
        }
        let record = unsafe { self.binding.actual_file_record(base, "project.sqlite") }?;
        if record.size != 0 {
            return Err(invalid());
        }
        let mut identity = self.identity.lock().map_err(|_| invalid())?;
        if identity.is_some() {
            return Err(invalid());
        }
        *identity = Some(record.identity);
        drop(identity);
        self.check()
    }
    pub fn finish_creation(&self) -> Result<()> {
        if let Some(published) = &self.published {
            published.recheck_new()?;
        }
        self.check()?;
        self.promoted.store(true, Ordering::Release);
        self.creating.store(false, Ordering::Release);
        self.check()
    }
    pub fn flush_namespace(&self) -> Result<()> {
        self.check()?;
        #[cfg(target_os = "linux")]
        {
            use std::os::unix::fs::OpenOptionsExt;
            for path in [
                self.binding.path("assets"),
                self.binding.path(".forma"),
                self.binding.path("."),
            ] {
                fs::OpenOptions::new()
                    .read(true)
                    .custom_flags(0x1_0000 | 0x2_0000)
                    .open(path)?
                    .sync_all()?;
            }
        }
        // Windows FULL sync plus cooperative guards is not universal power-loss durability.
        self.check()
    }
    pub fn promote(&self) -> Result<()> {
        self.check_active()?;
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
        if let Some(original) = &self.original {
            original.check_location()?;
        }
        if let Some(published) = &self.published {
            published.recheck_identity()?;
        }
        self.lease.recheck()?;
        if !self.binding.matches_directory(&self.root)?
            || !self
                .assets
                .matches_directory(&self.binding.path("assets"))?
        {
            return Err(invalid());
        }
        let identity = *self.identity.lock().map_err(|_| invalid())?;
        match identity {
            Some(identity) => {
                let record = self.binding.file_record("project.sqlite")?;
                if record.identity != identity
                    || platform::path_identity(&self.root.join("project.sqlite"))? != identity
                    || record.size > MAX_DATABASE_BYTES
                    || (!self.creating() && record.size < 512)
                {
                    return Err(invalid());
                }
            }
            None if self.creating() => self.absent("project.sqlite")?,
            None => return Err(invalid()),
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
        // Revocation prevents a new journal/transaction, but SQLite must finish
        // or roll back the already journaled operation using this live base.
        // The host rejects acceptance after close and resolves uncertain COMMIT.
        if self.journal_id()?.is_none() {
            self.check_active()?;
        }
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
        self.check_active()?;
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
            != self
                .identity
                .lock()
                .map_err(|_| invalid())?
                .ok_or_else(invalid)?
        {
            self.fail(2);
            return Err(invalid());
        }
        if self.creating() {
            return self.check();
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
