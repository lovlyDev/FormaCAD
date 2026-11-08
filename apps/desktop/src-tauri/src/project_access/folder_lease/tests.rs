use super::*;

#[cfg(any(
    windows,
    all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )
))]
#[test]
fn initialization_requires_a_preexisting_empty_selected_directory() {
    let temp = tempfile::tempdir().unwrap();
    let missing = temp.path().join("not-created-by-backend");
    assert!(FolderLease::initialize_empty(&missing).is_err());
    assert!(!missing.exists());
    let root = temp.path().join("selected-empty-directory");
    std::fs::create_dir(&root).unwrap();
    let lease = FolderLease::initialize_empty(&root).unwrap();
    lease.recheck().unwrap();
    let job = lease.clone();
    drop(lease);
    assert!(FolderLease::acquire(&root).is_err());
    job.recheck().unwrap();
    drop(job);
    let reopened = FolderLease::acquire(&root).unwrap();
    reopened.recheck().unwrap();
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
    assert!(root.join(".forma/writer.lock").is_file());
}

#[cfg(any(
    windows,
    all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )
))]
#[test]
fn nonempty_selected_directory_is_rejected_without_any_initialization_writes() {
    for kind in ["file", "directory"] {
        let root = tempfile::tempdir().unwrap();
        let existing = root.path().join("existing-user-entry");
        if kind == "file" {
            std::fs::write(&existing, b"original user data").unwrap();
        } else {
            std::fs::create_dir(&existing).unwrap();
        }
        assert!(FolderLease::initialize_empty(root.path()).is_err());
        assert!(!root.path().join(".forma").exists());
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
        if kind == "file" {
            assert_eq!(std::fs::read(existing).unwrap(), b"original user data");
        } else {
            assert_eq!(std::fs::read_dir(existing).unwrap().count(), 0);
        }
    }
}

#[cfg(any(
    windows,
    all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )
))]
#[test]
fn retained_assets_guard_keeps_writer_authority_and_uses_the_selected_root() {
    let root = tempfile::tempdir().unwrap();
    let lease = FolderLease::acquire(root.path()).unwrap();
    let database = lease.database_path();
    std::fs::write(&database, b"path identity probe, not a SQLite fixture").unwrap();
    assert_eq!(
        std::fs::read(root.path().join("project.sqlite")).unwrap(),
        b"path identity probe, not a SQLite fixture"
    );
    let assets = lease.assets_directory().unwrap();
    drop(lease);
    assert!(FolderLease::acquire(root.path()).is_err());
    std::fs::write(assets.path().join("probe"), b"pinned asset directory").unwrap();
    assets.recheck().unwrap();
    assert_eq!(
        std::fs::read(root.path().join("assets/probe")).unwrap(),
        b"pinned asset directory"
    );
    drop(assets);
    FolderLease::acquire(root.path()).unwrap();
}

#[cfg(any(
    windows,
    all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )
))]
#[test]
fn independent_sources_with_same_project_id_and_cloned_jobs_keep_separate_leases() {
    let left = tempfile::tempdir().unwrap();
    let right = tempfile::tempdir().unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    for root in [left.path(), right.path()] {
        std::fs::write(root.join("project-id"), &id).unwrap();
    }
    let first = FolderLease::acquire(left.path()).unwrap();
    let other_source = FolderLease::acquire(right.path()).unwrap();
    first.recheck().unwrap();
    other_source.recheck().unwrap();
    let error = FolderLease::acquire(left.path()).err().unwrap();
    assert_eq!(error.to_string(), "PROJECT_READ_ONLY");
    let job = first.clone();
    drop(first);
    assert!(FolderLease::acquire(left.path()).is_err());
    job.recheck().unwrap();
    drop(job);
    let reopened = FolderLease::acquire(left.path()).unwrap();
    reopened.recheck().unwrap();
    assert_eq!(
        std::fs::read(left.path().join("project-id")).unwrap(),
        id.as_bytes()
    );
    drop(reopened);
    drop(other_source);
    assert!(left.path().join(".forma/writer.lock").is_file());
    assert_eq!(
        std::fs::read_dir(left.path().join(".forma"))
            .unwrap()
            .count(),
        1
    );
}

#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
mod linux {
    use super::*;
    use std::os::unix::fs::symlink;

    #[test]
    fn initialized_root_remains_pinned_and_replacement_gets_no_writes() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("selected");
        std::fs::create_dir(&root).unwrap();
        let lease = FolderLease::initialize_empty(&root).unwrap();
        std::fs::rename(&root, temp.path().join("original-selected")).unwrap();
        std::fs::create_dir(&root).unwrap();
        assert!(lease.recheck().is_err());
        assert!(lease.assets_directory().is_err());
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 0);
        assert!(temp
            .path()
            .join("original-selected/.forma/writer.lock")
            .is_file());
    }

    #[test]
    fn initialization_refuses_a_linked_parent_without_creating_outside_entries() {
        let temp = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let outside_root = outside.path().join("selected");
        std::fs::create_dir(&outside_root).unwrap();
        symlink(outside.path(), temp.path().join("parent")).unwrap();
        assert!(FolderLease::initialize_empty(&temp.path().join("parent/selected")).is_err());
        assert_eq!(std::fs::read_dir(&outside_root).unwrap().count(), 0);
    }

    #[test]
    fn root_parent_inner_and_leaf_links_refuse_without_external_changes() {
        for target in ["root", "parent", "inner", "leaf"] {
            let temp = tempfile::tempdir().unwrap();
            let outside = tempfile::tempdir().unwrap();
            std::fs::write(outside.path().join("sentinel"), b"unchanged").unwrap();
            let parent = temp.path().join("parent");
            let root = parent.join("project");
            std::fs::create_dir_all(&root).unwrap();
            match target {
                "root" => {
                    std::fs::remove_dir(&root).unwrap();
                    symlink(outside.path(), &root).unwrap();
                }
                "parent" => {
                    std::fs::remove_dir_all(&parent).unwrap();
                    symlink(outside.path(), &parent).unwrap();
                }
                "inner" => {
                    symlink(outside.path(), root.join(".forma")).unwrap();
                }
                "leaf" => {
                    std::fs::create_dir(root.join(".forma")).unwrap();
                    symlink(
                        outside.path().join("sentinel"),
                        root.join(".forma/writer.lock"),
                    )
                    .unwrap();
                }
                _ => unreachable!(),
            }
            assert!(FolderLease::acquire(&root).is_err());
            assert_eq!(
                std::fs::read(outside.path().join("sentinel")).unwrap(),
                b"unchanged"
            );
            assert_eq!(std::fs::read_dir(outside.path()).unwrap().count(), 1);
        }
    }

    #[test]
    fn replaced_root_inner_and_lock_invalidate_the_original_guard() {
        for target in ["root", "inner", "lock"] {
            let temp = tempfile::tempdir().unwrap();
            let root = temp.path().join("project");
            std::fs::create_dir(&root).unwrap();
            let lease = FolderLease::acquire(&root).unwrap();
            match target {
                "root" => {
                    std::fs::rename(&root, temp.path().join("old-project")).unwrap();
                    std::fs::create_dir(&root).unwrap();
                }
                "inner" => {
                    std::fs::rename(root.join(".forma"), root.join("old-forma")).unwrap();
                    std::fs::create_dir(root.join(".forma")).unwrap();
                }
                "lock" => {
                    std::fs::rename(
                        root.join(".forma/writer.lock"),
                        root.join(".forma/old.lock"),
                    )
                    .unwrap();
                    std::fs::write(root.join(".forma/writer.lock"), b"").unwrap();
                }
                _ => unreachable!(),
            }
            assert!(lease.recheck().is_err());
        }
    }

    #[test]
    fn hardlinked_lock_is_refused_without_modifying_the_other_name() {
        let temp = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let sentinel = outside.path().join("sentinel");
        std::fs::write(&sentinel, b"unchanged").unwrap();
        std::fs::create_dir(temp.path().join(".forma")).unwrap();
        std::fs::hard_link(&sentinel, temp.path().join(".forma/writer.lock")).unwrap();
        assert!(FolderLease::acquire(temp.path()).is_err());
        assert_eq!(std::fs::read(sentinel).unwrap(), b"unchanged");
    }

    #[test]
    fn assets_links_and_replacement_cannot_redirect_retained_paths() {
        let temp = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let lease = FolderLease::acquire(temp.path()).unwrap();
        symlink(outside.path(), temp.path().join("assets")).unwrap();
        assert!(lease.assets_directory().is_err());
        std::fs::remove_file(temp.path().join("assets")).unwrap();
        let assets = lease.assets_directory().unwrap();
        std::fs::rename(
            temp.path().join("assets"),
            temp.path().join("original-assets"),
        )
        .unwrap();
        symlink(outside.path(), temp.path().join("assets")).unwrap();
        assert!(assets.recheck().is_err());
        std::fs::write(assets.path().join("anchor-proof"), b"original directory").unwrap();
        assert_eq!(
            std::fs::read(temp.path().join("original-assets/anchor-proof")).unwrap(),
            b"original directory"
        );
        assert_eq!(std::fs::read_dir(outside.path()).unwrap().count(), 0);
    }
}

#[cfg(windows)]
#[test]
fn windows_directory_and_lock_renames_are_blocked_until_last_clone_finishes() {
    let temp = tempfile::tempdir().unwrap();
    let parent = temp.path().join("parent");
    let root = parent.join("project");
    std::fs::create_dir_all(&root).unwrap();
    let lease = FolderLease::acquire(&root).unwrap();
    let assets = lease.assets_directory().unwrap();
    let job = lease.clone();
    drop(lease);
    assert!(std::fs::rename(&parent, temp.path().join("renamed-parent")).is_err());
    assert!(std::fs::rename(&root, parent.join("renamed-project")).is_err());
    assert!(std::fs::rename(root.join(".forma"), root.join("renamed-forma")).is_err());
    assert!(std::fs::rename(
        root.join(".forma/writer.lock"),
        root.join(".forma/renamed.lock")
    )
    .is_err());
    assert!(std::fs::rename(root.join("assets"), root.join("renamed-assets")).is_err());
    job.recheck().unwrap();
    drop(job);
    drop(assets);
    std::fs::rename(&root, parent.join("renamed-project")).unwrap();
}

#[cfg(not(any(
    windows,
    all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )
)))]
#[test]
fn unsupported_platform_has_no_weak_writer_fallback() {
    let temp = tempfile::tempdir().unwrap();
    assert_eq!(
        FolderLease::acquire(temp.path()).err().unwrap().to_string(),
        "PROJECT_ACCESS_UNAVAILABLE"
    );
    assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 0);
    assert_eq!(
        FolderLease::initialize_empty(temp.path())
            .err()
            .unwrap()
            .to_string(),
        "PROJECT_ACCESS_UNAVAILABLE"
    );
    assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 0);
}
