use crate::core::{AppError, Result};
use serde::Serialize;
use std::{
    collections::HashMap,
    fs::{File, OpenOptions, TryLockError},
    io::{Read, Write},
    path::Path,
    sync::{Arc, Mutex, Weak},
};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessStatus {
    pub project_id: String,
    pub mode: String,
    pub owner_pid: Option<u32>,
}
enum Session {
    Writer(Arc<File>),
    ReadOnly,
}
#[derive(Default)]
struct ProjectState {
    session: Option<Session>,
    in_flight: Weak<File>,
    read_epoch: Option<uuid::Uuid>,
}
#[derive(Default)]
pub struct AccessRegistry {
    // The map lock only resolves a per-project gate. Disk and lease operations
    // hold that gate, so a slow source cannot block unrelated project lifecycles.
    projects: Mutex<HashMap<String, Arc<Mutex<ProjectState>>>>,
}
/// Holding this guard prevents release during a nested commit or long-running task.
#[derive(Debug)]
pub struct WriteGuard {
    _file: Arc<File>,
}
impl AccessRegistry {
    fn project(&self, id: &str) -> Result<Arc<Mutex<ProjectState>>> {
        crate::security::valid_id(id)?;
        let mut projects = self
            .projects
            .lock()
            .map_err(|_| AppError::Invalid("PROJECT_ACCESS_UNAVAILABLE".into()))?;
        Ok(Arc::clone(projects.entry(id.into()).or_default()))
    }
    fn owner(root: &Path, id: &str) -> Option<u32> {
        let path =
            crate::security::guarded(root, Path::new(&format!(".project-access/{id}.owner.json")))
                .ok()?;
        let mut text = String::new();
        File::open(path)
            .ok()?
            .take(2048)
            .read_to_string(&mut text)
            .ok()?;
        serde_json::from_str::<serde_json::Value>(&text)
            .ok()?
            .get("pid")?
            .as_u64()?
            .try_into()
            .ok()
    }
    fn describe(root: &Path, id: &str, session: Option<&Session>) -> AccessStatus {
        let (mode, owner_pid) = match session {
            Some(Session::Writer(_)) => ("write", Some(std::process::id())),
            Some(Session::ReadOnly) => ("read_only", Self::owner(root, id)),
            None => ("closed", None),
        };
        AccessStatus {
            project_id: id.into(),
            mode: mode.into(),
            owner_pid,
        }
    }
    fn open(root: &Path, id: &str) -> Result<Session> {
        let path =
            crate::security::guarded(root, Path::new(&format!(".project-access/{id}.lock")))?;
        std::fs::create_dir_all(path.parent().unwrap())?;
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?;
        match file.try_lock() {
            Ok(()) => {
                file.set_len(0)?;
                let owner = serde_json::json!({"pid":std::process::id(),"sessionId":uuid::Uuid::new_v4().to_string(),"acquiredAt":chrono::Utc::now().to_rfc3339()});
                file.write_all(&serde_json::to_vec(&owner)?)?;
                file.sync_all()?;
                // Windows denies reading exclusively locked byte ranges. Keep informational
                // owner metadata in a separate bounded file; the OS lock remains authoritative.
                let temporary = crate::security::guarded(
                    root,
                    Path::new(&format!(
                        ".project-access/{id}.owner-{}.tmp",
                        uuid::Uuid::new_v4()
                    )),
                )?;
                let target = crate::security::guarded(
                    root,
                    Path::new(&format!(".project-access/{id}.owner.json")),
                )?;
                std::fs::write(&temporary, serde_json::to_vec(&owner)?)?;
                std::fs::rename(temporary, target)?;
                Ok(Session::Writer(Arc::new(file)))
            }
            Err(TryLockError::WouldBlock) => Ok(Session::ReadOnly),
            Err(TryLockError::Error(error)) => Err(error.into()),
        }
    }
    pub fn acquire(&self, root: &Path, id: &str) -> Result<AccessStatus> {
        let project = self.project(id)?;
        let mut state = project
            .lock()
            .map_err(|_| AppError::Invalid("PROJECT_ACCESS_UNAVAILABLE".into()))?;
        if matches!(state.session, Some(Session::Writer(_))) {
            return Ok(Self::describe(root, id, state.session.as_ref()));
        }
        let session = if let Some(file) = state.in_flight.upgrade() {
            Session::Writer(file)
        } else {
            Self::open(root, id)?
        };
        state.session = Some(session);
        if let Some(epoch) = &mut state.read_epoch {
            *epoch = uuid::Uuid::new_v4();
        }
        Ok(Self::describe(root, id, state.session.as_ref()))
    }
    pub fn status(&self, root: &Path, id: &str) -> Result<AccessStatus> {
        let project = self.project(id)?;
        let state = project
            .lock()
            .map_err(|_| AppError::Invalid("PROJECT_ACCESS_UNAVAILABLE".into()))?;
        Ok(Self::describe(root, id, state.session.as_ref()))
    }
    pub fn release(&self, id: &str) -> Result<()> {
        let project = self.project(id)?;
        let mut state = project
            .lock()
            .map_err(|_| AppError::Invalid("PROJECT_ACCESS_UNAVAILABLE".into()))?;
        state.session = None;
        if let Some(epoch) = &mut state.read_epoch {
            *epoch = uuid::Uuid::new_v4();
        }
        // Never unlink: an old live inode and recreated lock path could have simultaneous owners.
        Ok(())
    }
    /// Memory-only lifecycle token; never opens a lock file or grants write access.
    pub(crate) fn read_epoch(&self, id: &str) -> Result<uuid::Uuid> {
        let project = self.project(id)?;
        let mut state = project
            .lock()
            .map_err(|_| AppError::Invalid("PROJECT_ACCESS_UNAVAILABLE".into()))?;
        Ok(*state.read_epoch.get_or_insert_with(uuid::Uuid::new_v4))
    }
    pub fn ensure_write(&self, root: &Path, id: &str) -> Result<WriteGuard> {
        let project = self.project(id)?;
        let mut state = project
            .lock()
            .map_err(|_| AppError::Invalid("PROJECT_ACCESS_UNAVAILABLE".into()))?;
        let file = match state.session.as_ref() {
            Some(Session::Writer(file)) => Arc::clone(file),
            Some(Session::ReadOnly) => return Err(AppError::Invalid("PROJECT_READ_ONLY".into())),
            None => {
                if let Some(file) = state.in_flight.upgrade() {
                    file
                } else {
                    match Self::open(root, id)? {
                        Session::Writer(file) => file,
                        Session::ReadOnly => {
                            return Err(AppError::Invalid("PROJECT_READ_ONLY".into()))
                        }
                    }
                }
            }
        };
        state.in_flight = Arc::downgrade(&file);
        Ok(WriteGuard { _file: file })
    }
}

#[cfg(test)]
#[path = "registry_tests.rs"]
mod tests;
