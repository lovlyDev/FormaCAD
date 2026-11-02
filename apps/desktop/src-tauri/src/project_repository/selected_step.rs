//! One explicitly selected immutable input observation, not a legacy DB reader.
//! A STEP filename never grants access to another path or to project writes.
use super::location::LocationIdentity;
use crate::{
    core::{AppError, Result},
    models::ProjectFile,
};
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom},
    path::{Component, Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

#[cfg(target_os = "linux")]
#[path = "selected_step/linux.rs"]
mod platform;
#[cfg(windows)]
#[path = "selected_step/windows.rs"]
mod platform;

struct Input {
    file: Mutex<File>,
    path: PathBuf,
    parent: PathBuf,
    location: LocationIdentity,
    identity: platform::Identity,
    descriptor: ProjectFile,
    opening: String,
    active: AtomicBool,
}
pub(crate) struct SelectedStep {
    input: Arc<Input>,
}
pub(crate) struct StepSnapshot {
    input: Arc<Input>,
}

fn changed() -> AppError {
    AppError::Invalid("PROJECT_SNAPSHOT_CHANGED".into())
}
fn invalid() -> AppError {
    AppError::Invalid("PROJECT_FOLDER_ASSET_INVALID".into())
}

fn named(path: &Path) -> Result<std::fs::Metadata> {
    let metadata = std::fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(invalid());
    }
    platform::check_metadata(&metadata)?;
    if metadata.len() == 0 || metadata.len() > crate::artifacts::MAX_FILE_BYTES as u64 {
        return Err(invalid());
    }
    Ok(metadata)
}

fn digest(file: &mut File, expected: u64) -> Result<String> {
    file.seek(SeekFrom::Start(0))?;
    let mut hash = Sha256::new();
    let mut buffer = [0_u8; 32768];
    let mut total = 0_u64;
    loop {
        let size = file.read(&mut buffer)?;
        if size == 0 {
            break;
        }
        total = total
            .checked_add(size as u64)
            .filter(|total| *total <= expected)
            .ok_or_else(changed)?;
        hash.update(&buffer[..size]);
    }
    if total != expected {
        return Err(changed());
    }
    Ok(format!("{:x}", hash.finalize()))
}

impl Input {
    fn check(&self, file: &File) -> Result<()> {
        if !self.active.load(Ordering::Acquire)
            || !self.location.recheck(&self.parent, Path::new(""))?
        {
            return Err(changed());
        }
        let metadata = named(&self.path)?;
        if metadata.len() != self.descriptor.size
            || file.metadata()?.len() != self.descriptor.size
            || platform::identity(file)? != self.identity
            || platform::named_identity(&self.path, &metadata)? != self.identity
        {
            return Err(changed());
        }
        Ok(())
    }
}

impl SelectedStep {
    /// Trusted host entry for an explicitly selected file; no paths from a
    /// project manifest or imported CAD document may reach this constructor.
    pub(crate) fn open(path: &Path) -> Result<Self> {
        if !path.is_absolute()
            || path
                .components()
                .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
        {
            return Err(invalid());
        }
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(invalid)?;
        crate::security::file_name(name)?;
        if !path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                extension.eq_ignore_ascii_case("step") || extension.eq_ignore_ascii_case("stp")
            })
        {
            return Err(invalid());
        }
        let parent = path.parent().ok_or_else(invalid)?.to_path_buf();
        let location = LocationIdentity::capture(&parent, Path::new(""))?;
        let before = named(path)?;
        let mut options = OpenOptions::new();
        options.read(true);
        platform::configure(&mut options);
        let mut file = options.open(path)?;
        let identity = platform::identity(&file)?;
        if platform::named_identity(path, &before)? != identity
            || file.metadata()?.len() != before.len()
        {
            return Err(changed());
        }
        let hash = digest(&mut file, before.len())?;
        let input = Arc::new(Input {
            file: Mutex::new(file),
            path: path.into(),
            parent,
            location,
            identity,
            descriptor: ProjectFile {
                name: name.into(),
                size: before.len(),
                kind: "attachment".into(),
                data: None,
                sha256: Some(hash),
            },
            opening: uuid::Uuid::new_v4().to_string(),
            active: AtomicBool::new(true),
        });
        {
            let file = input.file.lock().map_err(|_| changed())?;
            input.check(&file)?;
        }
        Ok(Self { input })
    }

    pub(crate) fn capture(&self) -> Result<StepSnapshot> {
        let snapshot = StepSnapshot {
            input: Arc::clone(&self.input),
        };
        self.recheck(&snapshot)?;
        Ok(snapshot)
    }

    fn owns(&self, snapshot: &StepSnapshot) -> Result<()> {
        if !Arc::ptr_eq(&self.input, &snapshot.input) {
            return Err(changed());
        }
        Ok(())
    }

    pub(crate) fn recheck(&self, snapshot: &StepSnapshot) -> Result<()> {
        self.owns(snapshot)?;
        let mut file = self.input.file.lock().map_err(|_| changed())?;
        self.input.check(&file)?;
        if digest(&mut file, self.input.descriptor.size)?
            != *self.input.descriptor.sha256.as_ref().ok_or_else(invalid)?
        {
            return Err(changed());
        }
        self.input.check(&file)
    }

    pub(crate) fn read(&self, snapshot: &StepSnapshot) -> Result<Vec<u8>> {
        self.owns(snapshot)?;
        let mut file = self.input.file.lock().map_err(|_| changed())?;
        self.input.check(&file)?;
        file.seek(SeekFrom::Start(0))?;
        let mut bytes = vec![0; self.input.descriptor.size as usize];
        file.read_exact(&mut bytes)?;
        let mut extra = [0_u8; 1];
        if file.read(&mut extra)? != 0 {
            return Err(changed());
        }
        if bytes.len() as u64 != self.input.descriptor.size
            || crate::artifacts::digest(&bytes)
                != *self.input.descriptor.sha256.as_ref().ok_or_else(invalid)?
        {
            return Err(changed());
        }
        self.input.check(&file)?;
        Ok(bytes)
    }

    pub(crate) fn close(&self) {
        self.input.active.store(false, Ordering::Release);
    }
}
impl Drop for SelectedStep {
    fn drop(&mut self) {
        self.close();
    }
}
impl StepSnapshot {
    pub(crate) fn descriptor(&self) -> &ProjectFile {
        &self.input.descriptor
    }
    pub(crate) fn task_key(&self) -> &str {
        &self.input.opening
    }
}

#[cfg(test)]
mod tests;
