use crate::core::{AppError, AppState, Result};
use std::{
    future::Future,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
pub fn check(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Relaxed) {
        Err(AppError::Invalid("CAD_TASK_CANCELLED".into()))
    } else {
        Ok(())
    }
}
pub async fn run<T, F, Fut>(state: &AppState, id: &str, kind: &str, operation: F) -> Result<T>
where
    F: FnOnce(Arc<AtomicBool>) -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut tasks = state.tasks.lock().await;
        if tasks.contains_key(id) {
            return Err(AppError::Invalid(
                "A project task is already running".into(),
            ));
        }
        tasks.insert(id.into(), cancel.clone());
    }
    let result = async {
        let _permit = state.cad_tasks.acquire(id, kind, cancel.clone()).await?;
        check(&cancel)?;
        operation(cancel.clone()).await
    }
    .await;
    let mut tasks = state.tasks.lock().await;
    if tasks
        .get(id)
        .is_some_and(|current| Arc::ptr_eq(current, &cancel))
    {
        tasks.remove(id);
    }
    result
}
