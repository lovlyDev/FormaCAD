use super::LocationIdentity;
use std::{fs, path::Path};

#[test]
fn unchanged_existing_and_missing_locations_remain_current() {
    let temporary = tempfile::tempdir().unwrap();
    fs::create_dir(temporary.path().join("present")).unwrap();
    for name in ["present", "absent"] {
        let captured = LocationIdentity::capture(temporary.path(), Path::new(name)).unwrap();
        assert!(captured.recheck(temporary.path(), Path::new(name)).unwrap());
    }
}

#[test]
fn rename_and_identical_directory_replacement_are_stale() {
    let temporary = tempfile::tempdir().unwrap();
    let source = temporary.path().join("project");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("model.step"), b"same bytes").unwrap();
    let captured = LocationIdentity::capture(temporary.path(), Path::new("project")).unwrap();
    fs::rename(&source, temporary.path().join("old-project")).unwrap();
    assert!(!captured
        .recheck(temporary.path(), Path::new("project"))
        .unwrap());
    fs::create_dir(&source).unwrap();
    fs::write(source.join("model.step"), b"same bytes").unwrap();
    assert!(!captured
        .recheck(temporary.path(), Path::new("project"))
        .unwrap());
    assert_eq!(fs::read(source.join("model.step")).unwrap(), b"same bytes");
}

#[test]
fn appearance_of_a_missing_leaf_and_replacement_of_its_parent_are_stale() {
    let temporary = tempfile::tempdir().unwrap();
    fs::create_dir(temporary.path().join("parent")).unwrap();
    let relative = Path::new("parent/project");
    let absent = LocationIdentity::capture(temporary.path(), relative).unwrap();
    fs::create_dir(temporary.path().join(relative)).unwrap();
    assert!(!absent.recheck(temporary.path(), relative).unwrap());
    fs::remove_dir(temporary.path().join(relative)).unwrap();
    let absent = LocationIdentity::capture(temporary.path(), relative).unwrap();
    fs::rename(
        temporary.path().join("parent"),
        temporary.path().join("old-parent"),
    )
    .unwrap();
    fs::create_dir(temporary.path().join("parent")).unwrap();
    assert!(!absent.recheck(temporary.path(), relative).unwrap());
}

#[test]
fn root_replacement_and_different_location_are_stale() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("root");
    fs::create_dir(&root).unwrap();
    let captured = LocationIdentity::capture(&root, Path::new("absent")).unwrap();
    fs::rename(&root, temporary.path().join("old-root")).unwrap();
    fs::create_dir(&root).unwrap();
    assert!(!captured.recheck(&root, Path::new("absent")).unwrap());
    assert!(!captured
        .recheck(temporary.path(), Path::new("absent"))
        .unwrap());
}

#[test]
fn traversal_and_regular_files_are_rejected_without_writes() {
    let temporary = tempfile::tempdir().unwrap();
    fs::write(temporary.path().join("file"), b"unchanged").unwrap();
    for path in ["../outside", "file", "file/child"] {
        assert!(LocationIdentity::capture(temporary.path(), Path::new(path)).is_err());
    }
    assert_eq!(
        fs::read(temporary.path().join("file")).unwrap(),
        b"unchanged"
    );
}

#[cfg(unix)]
#[test]
fn symlink_leaf_and_root_are_rejected() {
    let temporary = tempfile::tempdir().unwrap();
    fs::create_dir(temporary.path().join("target")).unwrap();
    std::os::unix::fs::symlink(
        temporary.path().join("target"),
        temporary.path().join("link"),
    )
    .unwrap();
    assert!(LocationIdentity::capture(temporary.path(), Path::new("link")).is_err());
    assert!(
        LocationIdentity::capture(&temporary.path().join("link"), Path::new("absent")).is_err()
    );
}

#[cfg(windows)]
#[test]
fn junction_leaf_root_and_root_ancestor_are_rejected() {
    let temporary = tempfile::tempdir().unwrap();
    let target = temporary.path().join("target");
    let link = temporary.path().join("junction");
    fs::create_dir(&target).unwrap();
    fs::create_dir(target.join("nested")).unwrap();
    // A junction needs no symlink privilege; use only this fixture's paths.
    let output = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&link)
        .arg(&target)
        .output()
        .unwrap();
    assert!(output.status.success(), "fixture junction creation failed");
    let leaf_rejected = LocationIdentity::capture(temporary.path(), Path::new("junction")).is_err();
    let root_rejected = LocationIdentity::capture(&link, Path::new("absent")).is_err();
    let ancestor_rejected =
        LocationIdentity::capture(&link.join("nested"), Path::new("absent")).is_err();
    fs::remove_dir(&link).unwrap();
    assert!(leaf_rejected && root_rejected && ancestor_rejected);
    assert!(target.join("nested").is_dir());
}
