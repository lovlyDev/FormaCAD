//! Opening authority and weak virtual-name routing. No map mutex covers IO.
use super::{abi, invalid, platform};
use crate::{
    core::{AppError, Result},
    project_repository::location::LocationIdentity,
};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU8, Ordering},
        Arc, Mutex, OnceLock, Weak,
    },
};

pub(super) const MAX_DATABASE_BYTES: u64 = 512 * 1024 * 1024;
const PREFIX: &str = "/forma-ro/";
static SOURCES: OnceLock<Mutex<HashMap<String, Weak<Source>>>> = OnceLock::new();

pub(super) struct Source {
    root: PathBuf,
    location: LocationIdentity,
    pub binding: platform::Binding,
    identity: OnceLock<platform::FileIdentity>,
    fault: AtomicU8,
    token: String,
    #[cfg(test)]
    pub before_base_open: Mutex<Option<Box<dyn FnOnce() -> std::result::Result<(), ()> + Send>>>,
}

impl Source {
    pub fn capture(root: &Path) -> Result<Arc<Self>> {
        let root = std::path::absolute(root)?;
        let location = LocationIdentity::capture(&root, Path::new("assets"))?;
        let binding = platform::Binding::capture(&root)?;
        let source = Arc::new(Self {
            root,
            location,
            binding,
            identity: OnceLock::new(),
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
    pub fn check(&self) -> Result<()> {
        if let Some(error) = self.error() {
            return Err(error);
        }
        if !self.location.recheck(&self.root, Path::new("assets"))? {
            self.fail(2);
            return Err(invalid());
        }
        for suffix in ["-journal", "-wal", "-shm"] {
            match fs::symlink_metadata(self.binding.path(&format!("project.sqlite{suffix}"))) {
                Ok(_) => {
                    self.fail(1);
                    return Err(self.error().unwrap_or_else(invalid));
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
                Err(error) => return Err(error.into()),
            }
        }
        let current = platform::path_identity(&self.binding.path("project.sqlite"))?;
        if self
            .identity
            .get()
            .is_some_and(|expected| *expected != current)
            || current != platform::path_identity(&self.root.join("project.sqlite"))?
        {
            self.fail(2);
            return Err(invalid());
        }
        let metadata = fs::symlink_metadata(self.binding.path("project.sqlite"))?;
        if metadata.len() < 512 || metadata.len() > MAX_DATABASE_BYTES {
            self.fail(2);
            return Err(invalid());
        }
        if !self.location.recheck(&self.root, Path::new("assets"))? {
            self.fail(2);
            return Err(invalid());
        }
        Ok(())
    }
    /// SAFETY: base is a live SQLite file in xOpen or under SQLx lock_handle.
    pub unsafe fn attest(&self, base: *mut abi::File) -> Result<()> {
        self.check()?;
        // SAFETY: inherited caller guarantee; no raw OS database FD is opened.
        let actual = unsafe { self.binding.actual_identity(base, "project.sqlite") }?;
        if let Some(expected) = self.identity.get() {
            if *expected != actual {
                self.fail(2);
                return Err(invalid());
            }
        } else if self.identity.set(actual).is_err() && self.identity.get() != Some(&actual) {
            self.fail(2);
            return Err(invalid());
        }
        let mut header = [0_u8; 100];
        let mut size = 0_i64;
        // SAFETY: public base methods, initialized header/size buffers.
        let methods = unsafe { &*(*base).pMethods };
        let read = methods.xRead.ok_or_else(invalid)?;
        let file_size = methods.xFileSize.ok_or_else(invalid)?;
        if unsafe { read(base, header.as_mut_ptr().cast(), 100, 0) } != abi::OK
            || unsafe { file_size(base, &mut size) } != abi::OK
        {
            self.fail(2);
            return Err(invalid());
        }
        if &header[..16] != b"SQLite format 3\0"
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
    let tail = name.strip_prefix(PREFIX)?;
    let (token, file) = tail.split_once('/')?;
    if token.len() != 36
        || ![
            "project.sqlite",
            "project.sqlite-journal",
            "project.sqlite-wal",
            "project.sqlite-shm",
        ]
        .contains(&file)
    {
        return None;
    }
    let source = SOURCES.get()?.lock().ok()?.get(token)?.upgrade()?;
    Some((source, file))
}
