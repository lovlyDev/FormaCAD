use super::*;
use std::sync::{atomic::AtomicBool, Arc};

#[tokio::test]
async fn already_cancelled_work_never_enters_the_queue() {
    let gate = CadTaskGate::default();
    let id = uuid::Uuid::new_v4().to_string();
    let result = gate
        .acquire(&id, "model_build", Arc::new(AtomicBool::new(true)))
        .await;
    assert!(matches!(result, Err(error) if error.to_string().contains("CAD_TASK_CANCELLED")));
    assert!(gate.status().unwrap().is_empty());
}

#[tokio::test]
async fn cancelling_and_aborting_queued_futures_leaves_no_job_or_permit_leak() {
    let gate = Arc::new(CadTaskGate::default());
    let first = uuid::Uuid::new_v4().to_string();
    let second = uuid::Uuid::new_v4().to_string();
    let permit = gate
        .acquire(&first, "model_build", Arc::new(AtomicBool::new(false)))
        .await
        .unwrap();
    let second_gate = gate.clone();
    let waiting = tokio::spawn(async move {
        second_gate
            .acquire(&second, "model_build", Arc::new(AtomicBool::new(false)))
            .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            if gate.status().unwrap().len() == 2 {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    waiting.abort();
    assert!(matches!(waiting.await, Err(error) if error.is_cancelled()));
    assert_eq!(gate.status().unwrap().len(), 1);
    drop(permit);
    assert!(gate.status().unwrap().is_empty());
    let last = gate
        .acquire(&first, "section", Arc::new(AtomicBool::new(false)))
        .await
        .unwrap();
    drop(last);
    assert!(gate.status().unwrap().is_empty());
}
