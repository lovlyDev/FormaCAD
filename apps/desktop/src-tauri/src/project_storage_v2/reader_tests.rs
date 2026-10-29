use super::FolderSnapshot;
use crate::{
    models::{Project, ProjectFile},
    project_storage_v2 as storage,
};
use std::{fs, path::Path};

const ASSET: &[u8] = b"reader fixture attachment";

#[test]
fn metadata_tampering_without_a_manifest_change_invalidates_capture() {
    let folder = tempfile::tempdir().unwrap();
    let project = project();
    publish(folder.path(), &project, None);
    let captured = FolderSnapshot::capture(folder.path()).unwrap();
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(folder.path().join("manifest.json")).unwrap()).unwrap();
    let generation = manifest["generation"].as_str().unwrap();
    let path = folder
        .path()
        .join("metadata/generations")
        .join(generation)
        .join("project.json");
    let mut bytes = fs::read(&path).unwrap();
    bytes[0] = b'x';
    fs::write(&path, &bytes).unwrap();
    assert!(captured.recheck(folder.path()).is_err());
    assert!(captured
        .read_asset(folder.path(), &project.files[0])
        .is_err());
    assert_eq!(fs::read(&path).unwrap(), bytes);
}

fn project() -> Project {
    let mut project: Project = serde_json::from_value(serde_json::json!({
        "schemaVersion":1,"id":uuid::Uuid::new_v4().to_string(),"name":"Bounded reader fixture",
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

fn publish(root: &Path, project: &Project, history: Option<&[u8]>) {
    let mut staged = storage::stage_with(root, project, |_| Ok(ASSET.to_vec())).unwrap();
    if let Some(history) = history {
        storage::stage_history(root, &mut staged, history).unwrap();
    }
    storage::publish(root, staged).unwrap();
}

fn valid_history(project: &Project) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "schemaVersion":1,"state":crate::project_history::HistoryState::from_project(project).unwrap(),
        "events":[]
    })).unwrap()
}

fn asset(root: &Path, project: &Project) -> std::path::PathBuf {
    root.join("assets")
        .join(project.files[0].sha256.as_ref().unwrap())
}

#[test]
fn metadata_capture_does_not_claim_missing_asset_or_archive_integrity() {
    let folder = tempfile::tempdir().unwrap();
    let project = project();
    publish(folder.path(), &project, Some(b"not an archive"));
    fs::remove_file(asset(folder.path(), &project)).unwrap();
    let before = fs::read(folder.path().join("manifest.json")).unwrap();
    let captured = FolderSnapshot::capture(folder.path()).unwrap();
    assert_eq!(captured.project().id, project.id);
    assert!(captured
        .read_asset(folder.path(), &project.files[0])
        .is_err());
    assert!(captured.read_history(folder.path()).is_err());
    assert!(storage::read_folder(folder.path()).is_err());
    assert_eq!(
        fs::read(folder.path().join("manifest.json")).unwrap(),
        before
    );
    assert!(!asset(folder.path(), &project).exists());
}

#[test]
fn exact_asset_membership_and_explicit_archive_validation_succeed() {
    let folder = tempfile::tempdir().unwrap();
    let project = project();
    let history = valid_history(&project);
    publish(folder.path(), &project, Some(&history));
    let captured = FolderSnapshot::capture(folder.path()).unwrap();
    assert_eq!(
        captured
            .read_asset(folder.path(), &project.files[0])
            .unwrap(),
        ASSET
    );
    assert_eq!(
        captured.read_history(folder.path()).unwrap().unwrap(),
        history
    );
    let mut foreign = project.files[0].clone();
    foreign.name = "foreign-name.step".into();
    assert!(captured.read_asset(folder.path(), &foreign).is_err());
    assert_eq!(fs::read(asset(folder.path(), &project)).unwrap(), ASSET);
}

#[test]
fn asset_truncation_growth_checksum_mismatch_and_directory_are_rejected() {
    let folder = tempfile::tempdir().unwrap();
    let project = project();
    publish(folder.path(), &project, None);
    let captured = FolderSnapshot::capture(folder.path()).unwrap();
    let path = asset(folder.path(), &project);
    for bytes in [
        b"short".to_vec(),
        vec![b'x'; ASSET.len() + 1],
        vec![b'x'; ASSET.len()],
    ] {
        fs::write(&path, bytes).unwrap();
        assert!(captured
            .read_asset(folder.path(), &project.files[0])
            .is_err());
    }
    fs::remove_file(&path).unwrap();
    fs::create_dir(&path).unwrap();
    assert!(captured
        .read_asset(folder.path(), &project.files[0])
        .is_err());
    assert!(path.is_dir());
}

#[test]
fn manifest_drift_rejects_existing_capture_for_every_explicit_read() {
    let folder = tempfile::tempdir().unwrap();
    let mut project = project();
    publish(folder.path(), &project, None);
    let captured = FolderSnapshot::capture(folder.path()).unwrap();
    project.name = "Published later".into();
    publish(folder.path(), &project, None);
    let before = fs::read(folder.path().join("manifest.json")).unwrap();
    assert!(captured.recheck(folder.path()).is_err());
    assert!(captured
        .read_asset(folder.path(), &project.files[0])
        .is_err());
    assert!(captured.read_history(folder.path()).is_err());
    assert_eq!(
        FolderSnapshot::capture(folder.path())
            .unwrap()
            .project()
            .name,
        project.name
    );
    assert_eq!(
        fs::read(folder.path().join("manifest.json")).unwrap(),
        before
    );
}

fn rewrite_project(root: &Path, mutate: impl FnOnce(&mut Project)) {
    let mut manifest = storage::manifest_bytes(root, "manifest.json").unwrap();
    let entry = manifest
        .files
        .iter_mut()
        .find(|entry| entry.path == "project.json")
        .unwrap();
    let path = root.join(format!(
        "metadata/generations/{}/project.json",
        manifest.generation
    ));
    let mut project: Project = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    mutate(&mut project);
    let bytes = serde_json::to_vec(&project).unwrap();
    entry.size = bytes.len() as u64;
    entry.sha256 = crate::artifacts::digest(&bytes);
    fs::write(path, bytes).unwrap();
    fs::write(
        root.join("manifest.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
}

#[test]
fn rehashed_metadata_with_inline_data_or_mismatched_asset_reference_is_rejected() {
    for inline in [true, false] {
        let folder = tempfile::tempdir().unwrap();
        let project = project();
        publish(folder.path(), &project, None);
        rewrite_project(folder.path(), |project| {
            if inline {
                project.files[0].data = Some(crate::artifacts::data_url("model.step", ASSET));
            } else {
                project.files[0].size += 1;
            }
        });
        let before = fs::read(folder.path().join("manifest.json")).unwrap();
        assert!(FolderSnapshot::capture(folder.path()).is_err());
        assert_eq!(
            fs::read(folder.path().join("manifest.json")).unwrap(),
            before
        );
    }
}

#[test]
fn malformed_project_future_manifest_and_oversize_entry_are_rejected() {
    let folder = tempfile::tempdir().unwrap();
    let project = project();
    publish(folder.path(), &project, None);
    let manifest = storage::manifest_bytes(folder.path(), "manifest.json").unwrap();
    for (version, oversize) in [
        (manifest.storage_version + 1, false),
        (manifest.storage_version, true),
    ] {
        let mut bad = manifest.clone();
        bad.storage_version = version;
        if oversize {
            bad.files[0].size = storage::manifest::MAX_METADATA + 1;
        }
        fs::write(
            folder.path().join("manifest.json"),
            serde_json::to_vec(&bad).unwrap(),
        )
        .unwrap();
        assert!(FolderSnapshot::capture(folder.path()).is_err());
    }
    let mut malformed = manifest;
    let entry = malformed
        .files
        .iter_mut()
        .find(|entry| entry.path == "project.json")
        .unwrap();
    let path = folder.path().join(format!(
        "metadata/generations/{}/project.json",
        malformed.generation
    ));
    fs::write(path, b"not JSON").unwrap();
    entry.size = 8;
    entry.sha256 = crate::artifacts::digest(b"not JSON");
    fs::write(
        folder.path().join("manifest.json"),
        serde_json::to_vec(&malformed).unwrap(),
    )
    .unwrap();
    assert!(FolderSnapshot::capture(folder.path()).is_err());
}
