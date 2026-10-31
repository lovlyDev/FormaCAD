//! Per-project gates preserve lease semantics without blocking unrelated sources.
use super::AccessRegistry;
use std::{sync::mpsc, time::Duration};

#[test]
fn unrelated_project_completes_while_another_project_gate_is_held() {
    let root = tempfile::tempdir().unwrap();
    let registry = AccessRegistry::default();
    let blocked_id = uuid::Uuid::new_v4().to_string();
    let other_id = uuid::Uuid::new_v4().to_string();
    let project = registry.project(&blocked_id).unwrap();
    let held = project.lock().unwrap();
    let (send, receive) = mpsc::channel();
    std::thread::scope(|scope| {
        scope.spawn(|| {
            let epoch = registry.read_epoch(&other_id).unwrap();
            assert_eq!(
                registry.status(root.path(), &other_id).unwrap().mode,
                "closed"
            );
            assert_eq!(
                registry.acquire(root.path(), &other_id).unwrap().mode,
                "write"
            );
            assert_ne!(registry.read_epoch(&other_id).unwrap(), epoch);
            registry.release(&other_id).unwrap();
            assert_eq!(
                registry.status(root.path(), &other_id).unwrap().mode,
                "closed"
            );
            send.send(()).unwrap();
        });
        // Match the gate held during potentially slow filesystem/OS-lock work.
        // Release it even on failure before joining the scoped worker.
        let completed = receive.recv_timeout(Duration::from_secs(5));
        drop(held);
        completed.expect("an unrelated project must not wait for this project gate");
    });
}

#[test]
fn retained_write_guard_survives_release_and_reacquire_without_a_second_os_lease() {
    let root = tempfile::tempdir().unwrap();
    let registry = AccessRegistry::default();
    let contender = AccessRegistry::default();
    let id = uuid::Uuid::new_v4().to_string();
    assert_eq!(registry.acquire(root.path(), &id).unwrap().mode, "write");
    let epoch = registry.read_epoch(&id).unwrap();
    let guard = registry.ensure_write(root.path(), &id).unwrap();
    registry.release(&id).unwrap();
    assert_ne!(registry.read_epoch(&id).unwrap(), epoch);
    assert_eq!(registry.status(root.path(), &id).unwrap().mode, "closed");
    assert_eq!(
        contender.acquire(root.path(), &id).unwrap().mode,
        "read_only"
    );
    assert_eq!(registry.acquire(root.path(), &id).unwrap().mode, "write");
    let reacquired = registry.read_epoch(&id).unwrap();
    assert_eq!(registry.acquire(root.path(), &id).unwrap().mode, "write");
    assert_eq!(registry.read_epoch(&id).unwrap(), reacquired);
    registry.release(&id).unwrap();
    drop(guard);
    assert_eq!(contender.acquire(root.path(), &id).unwrap().mode, "write");
}

#[test]
fn transient_write_guard_does_not_open_session_or_invalidate_reader() {
    let root = tempfile::tempdir().unwrap();
    let registry = AccessRegistry::default();
    let id = uuid::Uuid::new_v4().to_string();
    let epoch = registry.read_epoch(&id).unwrap();
    let first = registry.ensure_write(root.path(), &id).unwrap();
    let second = registry.ensure_write(root.path(), &id).unwrap();
    assert_eq!(registry.status(root.path(), &id).unwrap().mode, "closed");
    assert_eq!(registry.read_epoch(&id).unwrap(), epoch);
    drop(first);
    drop(second);
    assert_eq!(registry.read_epoch(&id).unwrap(), epoch);
}
