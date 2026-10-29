use super::FolderRepository;
use crate::{
    models::{Project, ProjectFile},
    project_storage_v2 as storage,
};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

const ASSET: &[u8] = b"folder adapter fixture";

fn project() -> Project {
    let mut project: Project = serde_json::from_value(serde_json::json!({
        "schemaVersion":1,"id":uuid::Uuid::new_v4().to_string(),"name":"Folder repository fixture",
        "units":"mm","agent":"codex","pinned":false,"createdAt":"2026-10-29T00:00:00Z",
        "updatedAt":"2026-10-29T00:00:00Z","revisions":[],"currentRevision":null,
        "messages":[],"files":[],"exports":[]
    }))
    .unwrap();
    project.files.push(ProjectFile {
        name: "model.step".into(),
        kind: "attachment".into(),
        size: ASSET.len() as u64,
        data: None,
        sha256: Some(crate::artifacts::digest(ASSET)),
    });
    project
}

fn publish(root: &Path, project: &Project) {
    let mut staged = storage::stage_with(root, project, |_| Ok(ASSET.to_vec())).unwrap();
    let history = serde_json::to_vec(&serde_json::json!({ "schemaVersion":1,
        "state":crate::project_history::HistoryState::from_project(project).unwrap(), "events":[] })).unwrap();
    storage::stage_history(root, &mut staged, &history).unwrap();
    storage::publish(root, staged).unwrap();
}

fn files(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, current: &Path, result: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(current).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, result);
            } else {
                result.insert(
                    path.strip_prefix(root).unwrap().to_path_buf(),
                    fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut result = BTreeMap::new();
    walk(root, root, &mut result);
    result
}

fn copy_fixture(source: &Path, destination: &Path) {
    for (relative, bytes) in files(source) {
        let path = destination.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
}

#[test]
fn ordinary_metadata_asset_and_history_reads_create_no_files_or_database() {
    let folder = tempfile::tempdir().unwrap();
    let project = project();
    publish(folder.path(), &project);
    let before = files(folder.path());
    let repository = FolderRepository::open(folder.path()).unwrap();
    let snapshot = repository.capture().unwrap();
    assert_eq!(snapshot.project().id, project.id);
    assert_eq!(
        repository.read_asset(&snapshot, &project.files[0]).unwrap(),
        ASSET
    );
    let history = repository.read_history(&snapshot).unwrap().unwrap();
    crate::project_history::validate_archive(snapshot.project(), &history).unwrap();
    repository.recheck(&snapshot).unwrap();
    assert_eq!(files(folder.path()), before);
    repository.close();
    assert_eq!(files(folder.path()), before);
}

#[test]
fn identical_source_ids_and_bytes_in_distinct_roots_never_exchange_snapshots() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let project = project();
    publish(first.path(), &project);
    copy_fixture(first.path(), second.path());
    let before_first = files(first.path());
    let before_second = files(second.path());
    let a = FolderRepository::open(first.path()).unwrap();
    let b = FolderRepository::open(second.path()).unwrap();
    let a_snapshot = a.capture().unwrap();
    let b_snapshot = b.capture().unwrap();
    assert!(a.recheck(&b_snapshot).is_err());
    assert!(b.recheck(&a_snapshot).is_err());
    assert!(b.read_asset(&a_snapshot, &project.files[0]).is_err());
    assert!(a.read_history(&b_snapshot).is_err());
    a.recheck(&a_snapshot).unwrap();
    b.recheck(&b_snapshot).unwrap();
    a.close();
    b.recheck(&b_snapshot).unwrap();
    assert_eq!(files(first.path()), before_first);
    assert_eq!(files(second.path()), before_second);
}

#[test]
fn close_reopen_and_second_opening_at_same_path_cannot_refresh_old_authority() {
    let folder = tempfile::tempdir().unwrap();
    let project = project();
    publish(folder.path(), &project);
    let before = files(folder.path());
    let first = FolderRepository::open(folder.path()).unwrap();
    let snapshot = first.capture().unwrap();
    let concurrent = FolderRepository::open(folder.path()).unwrap();
    assert!(concurrent.recheck(&snapshot).is_err());
    first.close();
    assert!(first.capture().is_err());
    assert!(first.recheck(&snapshot).is_err());
    assert!(first.read_asset(&snapshot, &project.files[0]).is_err());
    let reopened = FolderRepository::open(folder.path()).unwrap();
    assert!(reopened.recheck(&snapshot).is_err());
    let fresh = reopened.capture().unwrap();
    reopened.recheck(&fresh).unwrap();
    assert_eq!(files(folder.path()), before);
}

#[test]
fn dropping_opener_closes_lifetime_even_while_snapshot_keeps_token_alive() {
    let folder = tempfile::tempdir().unwrap();
    let project = project();
    publish(folder.path(), &project);
    let snapshot = {
        let repository = FolderRepository::open(folder.path()).unwrap();
        repository.capture().unwrap()
    };
    assert!(!snapshot
        .opening
        .active
        .load(std::sync::atomic::Ordering::Acquire));
    let reopened = FolderRepository::open(folder.path()).unwrap();
    assert!(reopened.recheck(&snapshot).is_err());
    assert!(reopened.read_history(&snapshot).is_err());
}

#[test]
fn identical_directory_replacement_is_stale_for_existing_opening() {
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("project");
    let previous = parent.path().join("previous");
    fs::create_dir(&root).unwrap();
    let project = project();
    publish(&root, &project);
    let repository = FolderRepository::open(&root).unwrap();
    let snapshot = repository.capture().unwrap();
    fs::rename(&root, &previous).unwrap();
    fs::create_dir(&root).unwrap();
    copy_fixture(&previous, &root);
    let before = files(&root);
    assert!(repository.recheck(&snapshot).is_err());
    assert!(repository.capture().is_err());
    let replacement = FolderRepository::open(&root).unwrap();
    replacement
        .recheck(&replacement.capture().unwrap())
        .unwrap();
    assert_eq!(files(&root), before);
}

#[test]
fn missing_or_corrupt_asset_is_rejected_without_recovery_or_source_changes() {
    let folder = tempfile::tempdir().unwrap();
    let project = project();
    publish(folder.path(), &project);
    let repository = FolderRepository::open(folder.path()).unwrap();
    let asset = folder
        .path()
        .join("assets")
        .join(project.files[0].sha256.as_ref().unwrap());
    fs::write(&asset, vec![b'x'; ASSET.len()]).unwrap();
    let before = files(folder.path());
    let snapshot = repository.capture().unwrap();
    assert!(repository.read_asset(&snapshot, &project.files[0]).is_err());
    assert_eq!(files(folder.path()), before);
    fs::remove_file(&asset).unwrap();
    let before = files(folder.path());
    let snapshot = repository.capture().unwrap();
    assert!(repository.read_asset(&snapshot, &project.files[0]).is_err());
    assert_eq!(files(folder.path()), before);
}

#[test]
fn manifest_commit_stales_old_snapshot_and_new_project_id_cannot_replace_open_project() {
    let folder = tempfile::tempdir().unwrap();
    let mut project = project();
    publish(folder.path(), &project);
    let repository = FolderRepository::open(folder.path()).unwrap();
    let snapshot = repository.capture().unwrap();
    project.name = "Next saved generation".into();
    publish(folder.path(), &project);
    assert!(repository.recheck(&snapshot).is_err());
    assert_eq!(repository.capture().unwrap().project().name, project.name);
    project.id = uuid::Uuid::new_v4().to_string();
    publish(folder.path(), &project);
    let before = files(folder.path());
    assert!(repository.capture().is_err());
    assert_eq!(files(folder.path()), before);
}
