//! Descriptor retention is deterministic here; no fork timing or unsafe code.
use super::AccessRegistry;
use std::fs::{File, OpenOptions, TryLockError};

fn open(path: &std::path::Path) -> File {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .unwrap()
}

#[test]
fn closing_one_raw_descriptor_does_not_release_its_duplicated_lock() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("control.lock");
    let owner = open(&path);
    owner.try_lock().unwrap();
    let duplicate = owner.try_clone().unwrap();
    drop(owner);
    let contender = open(&path);
    assert!(matches!(
        contender.try_lock(),
        Err(TryLockError::WouldBlock)
    ));
    drop(duplicate);
    contender.try_lock().unwrap();
    contender.unlock().unwrap();
}

#[test]
fn final_parent_guard_unlocks_while_duplicate_lives_and_nested_guards_protect() {
    let root = tempfile::tempdir().unwrap();
    let registry = AccessRegistry::default();
    let contender = AccessRegistry::default();
    let third = AccessRegistry::default();
    let id = uuid::Uuid::new_v4().to_string();
    let first = registry.ensure_write(root.path(), &id).unwrap();
    let nested = registry.ensure_write(root.path(), &id).unwrap();
    let duplicate = first._lease.file.try_clone().unwrap();
    drop(first);
    assert!(contender.ensure_write(root.path(), &id).is_err());
    drop(nested);
    let new_owner = contender.ensure_write(root.path(), &id).unwrap();
    assert!(third.ensure_write(root.path(), &id).is_err());
    // Closing a stale duplicate must not release the new owner's independent lock.
    drop(duplicate);
    assert!(third.ensure_write(root.path(), &id).is_err());
    drop(new_owner);
    let final_owner = third.ensure_write(root.path(), &id).unwrap();
    drop(final_owner);
}
