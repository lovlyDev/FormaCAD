use crate::core::{AppError, Result};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskInfo {
    pub task_id: String,
    pub project_id: String,
    pub kind: String,
    pub state: String,
}
struct Job {
    info: TaskInfo,
    cancel: Arc<AtomicBool>,
}
struct Shared {
    semaphore: Arc<Semaphore>,
    jobs: Mutex<BTreeMap<String, Job>>,
}
pub struct CadTaskGate {
    shared: Arc<Shared>,
}
impl Default for CadTaskGate {
    fn default() -> Self {
        Self {
            shared: Arc::new(Shared {
                semaphore: Arc::new(Semaphore::new(1)),
                jobs: Mutex::new(BTreeMap::new()),
            }),
        }
    }
}
struct Registration {
    shared: Arc<Shared>,
    id: String,
}
impl Drop for Registration {
    fn drop(&mut self) {
        if let Ok(mut jobs) = self.shared.jobs.lock() {
            jobs.remove(&self.id);
        }
    }
}
/// Remove running status before returning the permit, including abort/error unwinding.
pub struct TaskPermit {
    _registration: Registration,
    _permit: OwnedSemaphorePermit,
}

impl CadTaskGate {
    pub async fn acquire(
        &self,
        project_id: &str,
        kind: &str,
        cancel: Arc<AtomicBool>,
    ) -> Result<TaskPermit> {
        crate::security::valid_id(project_id)?;
        if cancel.load(Ordering::Relaxed) {
            return Err(AppError::Invalid("CAD_TASK_CANCELLED".into()));
        }
        let id = uuid::Uuid::new_v4().to_string();
        self.shared
            .jobs
            .lock()
            .map_err(|_| AppError::Invalid("CAD_TASK_QUEUE_UNAVAILABLE".into()))?
            .insert(
                id.clone(),
                Job {
                    info: TaskInfo {
                        task_id: id.clone(),
                        project_id: project_id.into(),
                        kind: kind.into(),
                        state: "queued".into(),
                    },
                    cancel: cancel.clone(),
                },
            );
        let registration = Registration {
            shared: self.shared.clone(),
            id: id.clone(),
        };
        let permit = tokio::select! {
            biased;
            _ = wait_cancel(&cancel) => return Err(AppError::Invalid("CAD_TASK_CANCELLED".into())),
            result = self.shared.semaphore.clone().acquire_owned() => result.map_err(|_| AppError::Invalid("CAD_TASK_QUEUE_UNAVAILABLE".into()))?,
        };
        if cancel.load(Ordering::Relaxed) {
            return Err(AppError::Invalid("CAD_TASK_CANCELLED".into()));
        }
        {
            let mut jobs = self
                .shared
                .jobs
                .lock()
                .map_err(|_| AppError::Invalid("CAD_TASK_QUEUE_UNAVAILABLE".into()))?;
            if cancel.load(Ordering::Relaxed) {
                return Err(AppError::Invalid("CAD_TASK_CANCELLED".into()));
            }
            jobs.get_mut(&id)
                .ok_or_else(|| AppError::Invalid("CAD_TASK_QUEUE_UNAVAILABLE".into()))?
                .info
                .state = "running".into();
        }
        Ok(TaskPermit {
            _registration: registration,
            _permit: permit,
        })
    }
    pub fn status(&self) -> Result<Vec<TaskInfo>> {
        Ok(self
            .shared
            .jobs
            .lock()
            .map_err(|_| AppError::Invalid("CAD_TASK_QUEUE_UNAVAILABLE".into()))?
            .values()
            .map(|job| job.info.clone())
            .collect())
    }
    pub fn cancel(&self, task_id: &str) -> Result<()> {
        crate::security::valid_id(task_id)?;
        if let Some(job) = self
            .shared
            .jobs
            .lock()
            .map_err(|_| AppError::Invalid("CAD_TASK_QUEUE_UNAVAILABLE".into()))?
            .get(task_id)
        {
            job.cancel.store(true, Ordering::Relaxed);
        }
        Ok(())
    }
    pub fn cancel_project(&self, project_id: &str) -> Result<()> {
        crate::security::valid_id(project_id)?;
        for job in self
            .shared
            .jobs
            .lock()
            .map_err(|_| AppError::Invalid("CAD_TASK_QUEUE_UNAVAILABLE".into()))?
            .values()
        {
            if job.info.project_id == project_id {
                job.cancel.store(true, Ordering::Relaxed);
            }
        }
        Ok(())
    }
}
async fn wait_cancel(cancel: &AtomicBool) {
    loop {
        if cancel.load(Ordering::Relaxed) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}
