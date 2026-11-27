//! Capability and source lifecycle tests, independent from managed app SQL.
use super::*;
use crate::project_storage_v2 as storage;
use std::fs;

fn fixture(root: &Path) {
    let mut project: Project = serde_json::from_value(serde_json::json!({
        "schemaVersion":1,"id":uuid::Uuid::new_v4().to_string(),"name":"backup capability fixture",
        "units":"mm","agent":"codex","pinned":false,"createdAt":"2026-11-02T00:00:00Z",
        "updatedAt":"2026-11-02T00:00:00Z","revisions":[],"currentRevision":null,
        "messages":[],"files":[],"exports":[]
    }))
    .unwrap();
    let bytes = b"capability fixture CAS";
    project.files.push(ProjectFile {
        name: "source.step".into(),
        size: bytes.len() as u64,
        kind: "attachment".into(),
        data: None,
        sha256: Some(crate::artifacts::digest(bytes)),
    });
    let mut staged = storage::stage_with(root, &project, |_| Ok(bytes.to_vec())).unwrap();
    let history = serde_json::to_vec(&serde_json::json!({"schemaVersion":1,
        "state":crate::project_history::HistoryState::from_project(&project).unwrap(),"events":[]}))
    .unwrap();
    storage::stage_history(root, &mut staged, &history).unwrap();
    storage::publish(root, staged).unwrap();
}

#[test]
fn backup_capability_retains_original_opening_and_checks_independent_target_roots() {
    let root = tempfile::tempdir().unwrap();
    let backup_root = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    fixture(root.path());
    let source = FolderRepository::open(root.path()).unwrap();
    let original = source.capture().unwrap();
    let verified = source
        .backup_committed(&original, backup_root.path())
        .unwrap();
    verified.recheck().unwrap();
    verified.recheck_source(&source, &original).unwrap();
    assert_eq!(verified.project().id, original.project().id);
    assert_eq!(
        verified.read_history().unwrap(),
        source.read_history(&original).unwrap().unwrap()
    );
    let provenance = verified.provenance();
    assert_eq!(provenance.root, backup_root.path().canonicalize().unwrap());
    assert_eq!(provenance.project_id, original.project().id);
    assert_eq!(provenance.manifest_sha256, original.saved.manifest_sha256());
    assert_eq!(provenance.generation, original.saved.generation());
    verified.validate_target(target.path()).unwrap();
    assert!(verified.validate_target(root.path()).is_err());
    assert!(verified.validate_target(backup_root.path()).is_err());
    let nested = backup_root.path().join("nested");
    fs::create_dir(&nested).unwrap();
    assert!(verified.validate_target(&nested).is_err());
    let other = FolderRepository::open(root.path()).unwrap();
    assert!(verified
        .recheck_source(&other, &other.capture().unwrap())
        .is_err());
    source.close();
    assert!(verified.recheck_source(&source, &original).is_err());
    verified.recheck().unwrap();
    let reopened = FolderRepository::open(root.path()).unwrap();
    assert!(verified
        .recheck_source(&reopened, &reopened.capture().unwrap())
        .is_err());
}

#[test]
fn closed_source_nested_or_nonempty_backup_never_receives_published_manifest() {
    let root = tempfile::tempdir().unwrap();
    fixture(root.path());
    let source = FolderRepository::open(root.path()).unwrap();
    let original = source.capture().unwrap();
    let nested = root.path().join("backup");
    fs::create_dir(&nested).unwrap();
    assert!(source.backup_committed(&original, &nested).is_err());
    assert_eq!(fs::read_dir(&nested).unwrap().count(), 0);
    let nonempty = tempfile::tempdir().unwrap();
    fs::write(nonempty.path().join("keep.txt"), b"keep").unwrap();
    assert!(source.backup_committed(&original, nonempty.path()).is_err());
    assert_eq!(fs::read(nonempty.path().join("keep.txt")).unwrap(), b"keep");
    assert!(!nonempty.path().join("manifest.json").exists());
    source.close();
    let fresh_destination = tempfile::tempdir().unwrap();
    assert!(source
        .backup_committed(&original, fresh_destination.path())
        .is_err());
    assert_eq!(fs::read_dir(fresh_destination.path()).unwrap().count(), 0);
}

#[test]
fn source_asset_corruption_after_verified_backup_cannot_pass_original_boundary() {
    let root = tempfile::tempdir().unwrap();
    let backup_root = tempfile::tempdir().unwrap();
    fixture(root.path());
    let source = FolderRepository::open(root.path()).unwrap();
    let original = source.capture().unwrap();
    let verified = source
        .backup_committed(&original, backup_root.path())
        .unwrap();
    assert_eq!(
        verified.read_asset(&original.project().files[0]).unwrap(),
        b"capability fixture CAS"
    );
    let hash = original.project().files[0].sha256.as_ref().unwrap();
    fs::write(
        root.path().join("assets").join(hash),
        b"tampered source CAS",
    )
    .unwrap();
    // Metadata-only checks alone do not establish asset integrity.
    source.recheck(&original).unwrap();
    assert!(verified.recheck_source(&source, &original).is_err());
    verified.recheck().unwrap();
}

#[cfg(target_os = "linux")]
#[test]
fn byte_identical_backup_replacement_before_capability_mint_is_refused() {
    fn copy_tree(source: &Path, destination: &Path) {
        for entry in fs::read_dir(source).unwrap() {
            let entry = entry.unwrap();
            let target = destination.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                fs::create_dir(&target).unwrap();
                copy_tree(&entry.path(), &target);
            } else {
                fs::copy(entry.path(), target).unwrap();
            }
        }
    }
    let parent = tempfile::tempdir().unwrap();
    let source_root = parent.path().join("source");
    let destination = parent.path().join("backup");
    let replacement = parent.path().join("replacement");
    let moved = parent.path().join("original-backup");
    for root in [&source_root, &destination, &replacement] {
        fs::create_dir(root).unwrap();
    }
    fixture(&source_root);
    let source = FolderRepository::open(&source_root).unwrap();
    let original = source.capture().unwrap();
    let swap = destination.clone();
    let retained = moved.clone();
    let result = source.backup_committed_inner(
        &original,
        &destination,
        Some(Box::new(move || {
            copy_tree(&swap, &replacement);
            fs::rename(&swap, &retained).unwrap();
            fs::rename(&replacement, &swap).unwrap();
        })),
    );
    assert!(result.is_err());
    // Both folders contain valid bytes; their physical opening identity differs.
    assert_eq!(
        fs::read(destination.join("manifest.json")).unwrap(),
        fs::read(moved.join("manifest.json")).unwrap()
    );
    FolderRepository::open(&destination)
        .unwrap()
        .capture()
        .unwrap();
    FolderRepository::open(&moved).unwrap().capture().unwrap();
    source.recheck(&original).unwrap();
}
