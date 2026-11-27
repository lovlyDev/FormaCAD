//! Exact generation backup fixtures; no CAD geometry or hardware claim.
use super::*;
use crate::{
    models::{Project, ProjectFile},
    project_storage_v2 as storage,
};
use std::{collections::BTreeMap, fs, path::PathBuf};

fn fixture(root: &Path) -> String {
    let bytes = b"listed backup asset";
    let hash = crate::artifacts::digest(bytes);
    let mut project: Project = serde_json::from_value(serde_json::json!({
        "schemaVersion":1,"id":uuid::Uuid::new_v4().to_string(),"name":"exact backup fixture",
        "units":"mm","agent":"codex","pinned":false,"createdAt":"2026-11-02T00:00:00Z",
        "updatedAt":"2026-11-02T00:00:00Z","revisions":[],"currentRevision":null,
        "messages":[],"files":[],"exports":[]
    }))
    .unwrap();
    project.files.push(ProjectFile {
        name: "source.step".into(),
        kind: "attachment".into(),
        size: bytes.len() as u64,
        data: None,
        sha256: Some(hash),
    });
    let mut staged = storage::stage_with(root, &project, |_| Ok(bytes.to_vec())).unwrap();
    let history = serde_json::to_vec(&serde_json::json!({"schemaVersion":1,
        "state":crate::project_history::HistoryState::from_project(&project).unwrap(),"events":[]}))
    .unwrap();
    storage::stage_history(root, &mut staged, &history).unwrap();
    let project_path = root.join(format!(
        "metadata/generations/{}/project.json",
        staged.manifest.generation
    ));
    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(&project_path).unwrap()).unwrap();
    let raw = format!("\n{}\n", serde_json::to_string_pretty(&value).unwrap()).into_bytes();
    fs::write(&project_path, &raw).unwrap();
    let entry = staged
        .manifest
        .files
        .iter_mut()
        .find(|entry| entry.path == "project.json")
        .unwrap();
    entry.sha256 = crate::artifacts::digest(&raw);
    entry.size = raw.len() as u64;
    let extra = b"manifest entry outside Project.files";
    let extra_hash = crate::artifacts::digest(extra);
    fs::write(root.join("assets").join(&extra_hash), extra).unwrap();
    staged
        .manifest
        .files
        .push(crate::project_storage_v2::manifest::Entry {
            path: format!("attachments/{extra_hash}"),
            size: extra.len() as u64,
            sha256: extra_hash.clone(),
        });
    storage::publish(root, staged).unwrap();
    let manifest = fs::read(root.join("manifest.json")).unwrap();
    fs::write(
        root.join("manifest.json"),
        [b"\n".as_slice(), &manifest, b"\n"].concat(),
    )
    .unwrap();
    fs::write(root.join("unlisted-secret.txt"), b"do not copy").unwrap();
    fs::create_dir(root.join("unlisted-generation")).unwrap();
    fs::write(
        root.join("unlisted-generation/old.txt"),
        b"old unrelated bytes",
    )
    .unwrap();
    extra_hash
}

fn tree(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, directory: &Path, files: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, files);
            } else {
                files.insert(
                    path.strip_prefix(root).unwrap().into(),
                    fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut files = BTreeMap::new();
    walk(root, root, &mut files);
    files
}

#[test]
fn exact_backup_retains_raw_metadata_manifest_and_extra_entry_without_unlisted_tree() {
    let source = tempfile::tempdir().unwrap();
    let backup = tempfile::tempdir().unwrap();
    let extra_hash = fixture(source.path());
    let original = FolderSnapshot::capture(source.path()).unwrap();
    let before = tree(source.path());
    original
        .backup_committed(source.path(), backup.path(), || Ok(()))
        .unwrap();
    let captured = FolderSnapshot::capture(backup.path()).unwrap();
    captured.verify_all(backup.path()).unwrap();
    assert_eq!(captured.seal, original.seal);
    assert_eq!(
        fs::read(backup.path().join("manifest.json")).unwrap(),
        before[Path::new("manifest.json")]
    );
    for entry in &original.manifest.files {
        let relative = super::super::entry_relative(&original.manifest, &entry.path);
        assert_eq!(
            fs::read(backup.path().join(&relative)).unwrap(),
            before[Path::new(&relative)]
        );
    }
    assert!(backup.path().join("assets").join(extra_hash).is_file());
    assert!(!backup.path().join("unlisted-secret.txt").exists());
    assert!(!backup.path().join("unlisted-generation").exists());
    assert!(!backup.path().join("project.sqlite").exists());
    assert_eq!(tree(source.path()), before);
}

#[test]
fn full_verification_checks_entries_not_referenced_by_project_files() {
    let source = tempfile::tempdir().unwrap();
    let backup = tempfile::tempdir().unwrap();
    let extra_hash = fixture(source.path());
    let original = FolderSnapshot::capture(source.path()).unwrap();
    original
        .backup_committed(source.path(), backup.path(), || Ok(()))
        .unwrap();
    fs::write(
        backup.path().join("assets").join(extra_hash),
        b"corrupt extra entry",
    )
    .unwrap();
    let saved = FolderSnapshot::capture(backup.path()).unwrap();
    assert!(saved.verify_all(backup.path()).is_err());
}

#[test]
fn unknown_project_fields_are_refused_without_reserving_a_backup() {
    let source = tempfile::tempdir().unwrap();
    let backup = tempfile::tempdir().unwrap();
    fixture(source.path());
    let mut manifest: storage::Manifest =
        serde_json::from_slice(&fs::read(source.path().join("manifest.json")).unwrap()).unwrap();
    let entry = manifest
        .files
        .iter_mut()
        .find(|entry| entry.path == "project.json")
        .unwrap();
    let path = source.path().join(format!(
        "metadata/generations/{}/project.json",
        manifest.generation
    ));
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    value["futureExtra"] = serde_json::json!({"unsupported":true});
    let bytes = serde_json::to_vec(&value).unwrap();
    fs::write(path, &bytes).unwrap();
    entry.size = bytes.len() as u64;
    entry.sha256 = crate::artifacts::digest(&bytes);
    fs::write(
        source.path().join("manifest.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    let before = tree(source.path());
    assert!(FolderSnapshot::capture(source.path()).is_err());
    assert_eq!(fs::read_dir(backup.path()).unwrap().count(), 0);
    assert_eq!(tree(source.path()), before);
}

#[test]
fn missing_history_and_corrupt_source_refuse_before_destination_reservation() {
    for remove_history in [false, true] {
        let source = tempfile::tempdir().unwrap();
        let backup = tempfile::tempdir().unwrap();
        let extra = fixture(source.path());
        if remove_history {
            let mut manifest: storage::Manifest =
                serde_json::from_slice(&fs::read(source.path().join("manifest.json")).unwrap())
                    .unwrap();
            manifest.files.retain(|entry| entry.path != "history.json");
            fs::write(
                source.path().join("manifest.json"),
                serde_json::to_vec(&manifest).unwrap(),
            )
            .unwrap();
        } else {
            fs::write(source.path().join("assets").join(extra), b"bad source").unwrap();
        }
        let original = FolderSnapshot::capture(source.path()).unwrap();
        assert!(original
            .backup_committed(source.path(), backup.path(), || Ok(()))
            .is_err());
        assert_eq!(fs::read_dir(backup.path()).unwrap().count(), 0);
    }
}

#[test]
fn late_failure_preserves_published_backup_and_its_known_seal() {
    let source = tempfile::tempdir().unwrap();
    let backup = tempfile::tempdir().unwrap();
    fixture(source.path());
    let original = FolderSnapshot::capture(source.path()).unwrap();
    let error = original
        .backup_committed(source.path(), backup.path(), || {
            if backup.path().join("manifest.json").exists() {
                Err(changed())
            } else {
                Ok(())
            }
        })
        .unwrap_err();
    assert!(error.to_string().contains(original.manifest_sha256()));
    assert!(error.to_string().contains(original.generation()));
    assert!(backup.path().join("manifest.json").is_file());
    FolderSnapshot::capture(backup.path())
        .unwrap()
        .verify_all(backup.path())
        .unwrap();
}

#[test]
fn output_leaf_create_new_refuses_existing_content() {
    let backup = tempfile::tempdir().unwrap();
    let lease = FolderLease::initialize_empty(backup.path()).unwrap();
    let writer = io::BackupTree::create(lease, &uuid::Uuid::new_v4().to_string()).unwrap();
    writer.write_entry("project.json", b"first").unwrap();
    assert!(writer.write_entry("project.json", b"second").is_err());
    writer.publish_manifest(b"first manifest").unwrap();
    assert!(writer.publish_manifest(b"second manifest").is_err());
    assert_eq!(
        fs::read(backup.path().join("manifest.json")).unwrap(),
        b"first manifest"
    );
}

#[cfg(target_os = "linux")]
#[test]
fn raced_symlink_leaf_is_refused_without_touching_external_bytes() {
    let backup = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    let generation = uuid::Uuid::new_v4().to_string();
    let writer = io::BackupTree::create(
        FolderLease::initialize_empty(backup.path()).unwrap(),
        &generation,
    )
    .unwrap();
    let external_file = external.path().join("keep.json");
    fs::write(&external_file, b"external original").unwrap();
    std::os::unix::fs::symlink(
        &external_file,
        backup
            .path()
            .join(format!("metadata/generations/{generation}/project.json")),
    )
    .unwrap();
    assert!(writer
        .write_entry("project.json", b"must not write")
        .is_err());
    assert_eq!(fs::read(external_file).unwrap(), b"external original");
}

#[test]
fn retained_generation_directory_is_never_replaced_by_a_fresh_path() {
    let backup = tempfile::tempdir().unwrap();
    let generation = uuid::Uuid::new_v4().to_string();
    let writer = io::BackupTree::create(
        FolderLease::initialize_empty(backup.path()).unwrap(),
        &generation,
    )
    .unwrap();
    let current = backup
        .path()
        .join(format!("metadata/generations/{generation}"));
    let moved = backup.path().join("metadata/generations/moved");
    writer.recheck().unwrap();
    #[cfg(target_os = "linux")]
    {
        fs::rename(&current, &moved).unwrap();
        fs::create_dir(&current).unwrap();
        assert!(writer.write_entry("project.json", b"stale output").is_err());
        assert!(!current.join("project.json").exists());
        assert!(!moved.join("project.json").exists());
    }
    #[cfg(windows)]
    assert!(
        fs::rename(&current, &moved).is_err(),
        "retained directory must deny delete sharing"
    );
}
