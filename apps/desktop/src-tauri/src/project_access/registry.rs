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
struct RegistryState {
    sessions: HashMap<String, Session>,
    in_flight: HashMap<String, Weak<File>>,
}
#[derive(Default)]
pub struct AccessRegistry {
    state: Mutex<RegistryState>,
}
/// Holding this guard prevents release during a nested commit or long-running task.
#[derive(Debug)]
pub struct WriteGuard {
    _file: Arc<File>,
}
impl AccessRegistry {
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
        crate::security::valid_id(id)?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| AppError::Invalid("PROJECT_ACCESS_UNAVAILABLE".into()))?;
        if matches!(state.sessions.get(id), Some(Session::Writer(_))) {
            return Ok(Self::describe(root, id, state.sessions.get(id)));
        }
        let session = if let Some(file) = state.in_flight.get(id).and_then(Weak::upgrade) {
            Session::Writer(file)
        } else {
            Self::open(root, id)?
        };
        state.sessions.insert(id.into(), session);
        Ok(Self::describe(root, id, state.sessions.get(id)))
    }
    pub fn status(&self, root: &Path, id: &str) -> Result<AccessStatus> {
        crate::security::valid_id(id)?;
        let state = self
            .state
            .lock()
            .map_err(|_| AppError::Invalid("PROJECT_ACCESS_UNAVAILABLE".into()))?;
        Ok(Self::describe(root, id, state.sessions.get(id)))
    }
    pub fn release(&self, id: &str) -> Result<()> {
        crate::security::valid_id(id)?;
        self.state
            .lock()
            .map_err(|_| AppError::Invalid("PROJECT_ACCESS_UNAVAILABLE".into()))?
            .sessions
            .remove(id);
        // Never unlink: an old live inode and recreated lock path could have simultaneous owners.
        Ok(())
    }
    pub fn ensure_write(&self, root: &Path, id: &str) -> Result<WriteGuard> {
        crate::security::valid_id(id)?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| AppError::Invalid("PROJECT_ACCESS_UNAVAILABLE".into()))?;
        let file = match state.sessions.get(id) {
            Some(Session::Writer(file)) => Arc::clone(file),
            Some(Session::ReadOnly) => return Err(AppError::Invalid("PROJECT_READ_ONLY".into())),
            None => {
                if let Some(file) = state.in_flight.get(id).and_then(Weak::upgrade) {
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
        state
            .in_flight
            .retain(|_, reference| reference.strong_count() > 0);
        state.in_flight.insert(id.into(), Arc::downgrade(&file));
        Ok(WriteGuard { _file: file })
    }
}
