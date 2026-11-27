//! File selection/lifecycle tests. Bytes are synthetic, not validated geometry.
use super::*;
use std::fs;

#[test]
fn capture_read_and_reopen_are_bound_to_one_opening_without_source_writes() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("Part.STEP");
    let bytes = b"synthetic selected STEP input";
    fs::write(&path, bytes).unwrap();
    let source = SelectedStep::open(&path).unwrap();
    let saved = source.capture().unwrap();
    assert_eq!(saved.descriptor().name, "Part.STEP");
    assert_eq!(saved.descriptor().size, bytes.len() as u64);
    assert_eq!(
        saved.descriptor().sha256.as_deref(),
        Some(crate::artifacts::digest(bytes).as_str())
    );
    crate::security::valid_id(saved.task_key()).unwrap();
    assert_eq!(source.read(&saved).unwrap(), bytes);
    let other = SelectedStep::open(&path).unwrap();
    let current = other.capture().unwrap();
    assert_ne!(saved.task_key(), current.task_key());
    assert!(other.recheck(&saved).is_err());
    source.close();
    assert!(source.read(&saved).is_err());
    assert!(source.capture().is_err());
    assert_eq!(other.read(&current).unwrap(), bytes);
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
}

#[test]
fn missing_empty_wrong_extension_directory_and_oversize_inputs_are_refused() {
    let root = tempfile::tempdir().unwrap();
    assert!(SelectedStep::open(&root.path().join("missing.step")).is_err());
    let empty = root.path().join("empty.step");
    fs::write(&empty, b"").unwrap();
    assert!(SelectedStep::open(&empty).is_err());
    let wrong = root.path().join("model.txt");
    fs::write(&wrong, b"input").unwrap();
    assert!(SelectedStep::open(&wrong).is_err());
    let directory = root.path().join("directory.step");
    fs::create_dir(&directory).unwrap();
    assert!(SelectedStep::open(&directory).is_err());
    let huge = root.path().join("oversize.stp");
    File::create(&huge)
        .unwrap()
        .set_len(crate::artifacts::MAX_FILE_BYTES as u64 + 1)
        .unwrap();
    assert!(SelectedStep::open(&huge).is_err());
    assert!(SelectedStep::open(Path::new("relative.step")).is_err());
}

#[test]
fn hardlink_alias_is_refused_without_changing_either_name() {
    let root = tempfile::tempdir().unwrap();
    let original = root.path().join("original.step");
    let alias = root.path().join("alias.step");
    fs::write(&original, b"must remain").unwrap();
    fs::hard_link(&original, &alias).unwrap();
    assert!(SelectedStep::open(&original).is_err());
    assert!(SelectedStep::open(&alias).is_err());
    assert_eq!(fs::read(&original).unwrap(), b"must remain");
    assert_eq!(fs::read(&alias).unwrap(), b"must remain");
}

#[cfg(target_os = "linux")]
#[test]
fn changed_bytes_and_replaced_path_never_refresh_the_captured_source() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("model.step");
    fs::write(&path, b"initial selected input").unwrap();
    let source = SelectedStep::open(&path).unwrap();
    let saved = source.capture().unwrap();
    fs::write(&path, b"changed selected input").unwrap();
    assert!(source.read(&saved).is_err());
    assert!(source.recheck(&saved).is_err());
    fs::write(&path, b"initial selected input").unwrap();
    source.recheck(&saved).unwrap(); // Optimistic content checks, not a write epoch.
    fs::rename(&path, root.path().join("moved.step")).unwrap();
    fs::write(&path, b"initial selected input").unwrap();
    assert!(source.recheck(&saved).is_err());
    assert!(source.capture().is_err());
}

#[cfg(target_os = "linux")]
#[test]
fn symlink_leaf_and_parent_are_refused() {
    let root = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    let path = external.path().join("model.step");
    fs::write(&path, b"selected bytes").unwrap();
    let linked = root.path().join("linked.step");
    std::os::unix::fs::symlink(&path, &linked).unwrap();
    assert!(SelectedStep::open(&linked).is_err());
    let parent = root.path().join("linked-root");
    std::os::unix::fs::symlink(external.path(), &parent).unwrap();
    assert!(SelectedStep::open(&parent.join("model.step")).is_err());
    assert_eq!(fs::read(path).unwrap(), b"selected bytes");
}

#[cfg(windows)]
#[test]
fn retained_source_handle_denies_writes_and_delete_sharing() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("model.step");
    fs::write(&path, b"selected bytes").unwrap();
    let source = SelectedStep::open(&path).unwrap();
    let saved = source.capture().unwrap();
    assert!(fs::write(&path, b"overwrite").is_err());
    assert!(fs::rename(&path, root.path().join("moved.step")).is_err());
    source.recheck(&saved).unwrap();
}
