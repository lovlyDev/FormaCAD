//! Synthetic saved-model migration; real geometry is covered by the CAD host.
use super::*;
use crate::{models::Project, project_history::Direction, project_storage_v2 as storage};
use std::{collections::BTreeMap, fs, path::PathBuf};

const A: &[u8] = b"migration synthetic STEP A";
const B: &[u8] = b"migration synthetic STEP B";

fn fixture(root: &Path) -> FolderRepository {
    let first = uuid::Uuid::new_v4().to_string();
    let second = uuid::Uuid::new_v4().to_string();
    let project: Project = serde_json::from_value(serde_json::json!({
        "schemaVersion":1,"id":uuid::Uuid::new_v4().to_string(),"name":"V2 migration fixture",
        "units":"mm","agent":"codex","pinned":false,"createdAt":"2026-11-02T00:00:00Z",
        "updatedAt":"2026-11-02T00:00:00Z","currentRevision":second,"messages":[],"exports":[],
        "files":[
            {"name":"a.step","size":A.len(),"kind":"model","sha256":crate::artifacts::digest(A)},
            {"name":"b.step","size":B.len(),"kind":"model","sha256":crate::artifacts::digest(B)}
        ],
        "revisions":[
            {"id":first,"parent":null,"createdAt":"2026-11-02T00:00:00Z","prompt":"baseline A","source":"a.step",
             "parameters":{"kind":"box","width":40,"depth":20,"height":10,"thickness":2,"holeDiameter":0,"holes":0}},
            {"id":second,"parent":first,"createdAt":"2026-11-02T00:00:00Z","prompt":"baseline B","source":"b.step",
             "parameters":{"kind":"box","width":80,"depth":20,"height":10,"thickness":2,"holeDiameter":0,"holes":0}}
        ]
    })).unwrap();
    let mut staged = storage::stage_with(root, &project, |file| {
        Ok(if file.name == "a.step" { A } else { B }.to_vec())
    })
    .unwrap();
    // An explicit portable baseline archive retains the saved parent-chain cursor.
    let history = serde_json::to_vec(&serde_json::json!({
        "schemaVersion":1,
        "state":crate::project_history::HistoryState::from_project(&project).unwrap(),
        "events":[]
    }))
    .unwrap();
    storage::stage_history(root, &mut staged, &history).unwrap();
    storage::publish(root, staged).unwrap();
    FolderRepository::open(root).unwrap()
}

fn tree(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, current: &Path, result: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(current).unwrap() {
            let path = entry.unwrap().path();
            let metadata = fs::symlink_metadata(&path).unwrap();
            assert!(!metadata.file_type().is_symlink());
            if metadata.is_dir() {
                walk(root, &path, result);
            } else {
                let bytes = if metadata.len() == 0 {
                    vec![]
                } else {
                    fs::read(&path).unwrap()
                };
                result.insert(path.strip_prefix(root).unwrap().into(), bytes);
            }
        }
    }
    let mut result = BTreeMap::new();
    walk(root, root, &mut result);
    result
}

fn assert_send<T: Send>(_: T) {}

#[tokio::test]
async fn verified_backup_precedes_conversion_and_new_process_can_read_and_redo() {
    let source_root = tempfile::tempdir().unwrap();
    let backup_root = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    let source = fixture(source_root.path());
    let original = source.capture().unwrap();
    let before = tree(source_root.path());
    let backup = source
        .backup_committed(&original, backup_root.path())
        .unwrap();
    assert!(tree(target.path()).is_empty());
    assert_send(migrate_verified_v2(
        &source,
        &original,
        &backup,
        target.path(),
    ));
    let report = migrate_verified_v2(&source, &original, &backup, target.path()).await;
    assert_eq!(
        report.backup.manifest_sha256,
        backup.provenance().manifest_sha256
    );
    let saved = match report.target.unwrap() {
        format::CommitOutcome::Committed(saved) => saved,
        other => panic!("migration not confirmed: {other:?}"),
    };
    assert_eq!(
        serde_json::to_value(saved.project()).unwrap(),
        serde_json::to_value(original.project()).unwrap()
    );
    let session = FolderSession::open(target.path()).await.unwrap();
    let edited = session.capture().await.unwrap();
    assert!(matches!(
        session
            .apply_history(&edited, Direction::Undo)
            .await
            .unwrap(),
        format::CommitOutcome::Committed(_)
    ));
    session.close();
    drop(session);
    let nonce = uuid::Uuid::new_v4().to_string();
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "project_repository::sqlite_folder::migration::tests::migration_child",
            "--nocapture",
        ])
        .env("FORMA_MIGRATION_TARGET", target.path())
        .env("FORMA_MIGRATION_BACKUP", backup_root.path())
        .env("FORMA_MIGRATION_PROJECT", &original.project().id)
        .env("FORMA_MIGRATION_NONCE", &nonce)
        .output()
        .unwrap();
    assert!(
        child.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&child.stdout),
        String::from_utf8_lossy(&child.stderr)
    );
    assert!(String::from_utf8_lossy(&child.stdout)
        .lines()
        .any(|line| line == format!("MIGRATION_REOPEN_OK:{nonce}")));
    let reopened = FolderSession::open(target.path()).await.unwrap();
    let undone = reopened.capture().await.unwrap();
    assert!(matches!(
        reopened
            .apply_history(&undone, Direction::Redo)
            .await
            .unwrap(),
        format::CommitOutcome::Committed(_)
    ));
    let redone = reopened.capture().await.unwrap();
    assert_eq!(
        reopened
            .read_asset(&redone, &redone.project().files[1])
            .await
            .unwrap(),
        B
    );
    source.recheck(&original).unwrap();
    backup.recheck().unwrap();
    assert_eq!(tree(source_root.path()), before);
}

#[tokio::test]
#[ignore = "exact fresh-process migration probe"]
async fn migration_child() {
    let target = PathBuf::from(std::env::var_os("FORMA_MIGRATION_TARGET").expect("parent target"));
    let backup_root =
        PathBuf::from(std::env::var_os("FORMA_MIGRATION_BACKUP").expect("parent backup"));
    let expected = std::env::var("FORMA_MIGRATION_PROJECT").unwrap();
    let backup = FolderRepository::open(&backup_root).unwrap();
    let captured = backup.capture().unwrap();
    assert_eq!(captured.project().id, expected);
    assert_eq!(
        backup
            .read_asset(&captured, &captured.project().files[1])
            .unwrap(),
        B
    );
    assert!(backup.read_history(&captured).unwrap().is_some());
    let session = FolderSession::open(&target).await.unwrap();
    let saved = session.capture().await.unwrap();
    assert_eq!(saved.project().id, expected);
    assert!(saved.history_state().can_redo());
    assert_eq!(
        session
            .read_asset(&saved, &saved.project().files[0])
            .await
            .unwrap(),
        A
    );
    println!(
        "\nMIGRATION_REOPEN_OK:{}",
        std::env::var("FORMA_MIGRATION_NONCE").unwrap()
    );
}

#[tokio::test]
async fn corrupted_backup_and_stale_source_never_start_target_and_keep_receipt() {
    for fault in ["closed-source", "corrupt-backup", "corrupt-source"] {
        let source_root = tempfile::tempdir().unwrap();
        let backup_root = tempfile::tempdir().unwrap();
        let target = tempfile::tempdir().unwrap();
        let source = fixture(source_root.path());
        let original = source.capture().unwrap();
        let backup = source
            .backup_committed(&original, backup_root.path())
            .unwrap();
        if fault == "corrupt-backup" {
            fs::write(
                backup_root
                    .path()
                    .join("assets")
                    .join(crate::artifacts::digest(A)),
                b"corrupt",
            )
            .unwrap();
        } else if fault == "corrupt-source" {
            fs::write(
                source_root
                    .path()
                    .join("assets")
                    .join(crate::artifacts::digest(A)),
                b"corrupt source",
            )
            .unwrap();
        } else {
            source.close();
        }
        let before = tree(source_root.path());
        let report = migrate_verified_v2(&source, &original, &backup, target.path()).await;
        assert!(report.target.is_err());
        assert_eq!(
            report.backup.manifest_sha256,
            backup.provenance().manifest_sha256
        );
        assert!(report.backup.root.join("manifest.json").is_file());
        assert!(tree(target.path()).is_empty());
        assert_eq!(tree(source_root.path()), before);
    }
}

#[tokio::test]
async fn target_refusal_retains_verified_backup_and_source() {
    let source_root = tempfile::tempdir().unwrap();
    let backup_root = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    let source = fixture(source_root.path());
    let original = source.capture().unwrap();
    let backup = source
        .backup_committed(&original, backup_root.path())
        .unwrap();
    fs::write(target.path().join("keep.txt"), b"foreign destination").unwrap();
    let source_before = tree(source_root.path());
    let target_before = tree(target.path());
    let report = migrate_verified_v2(&source, &original, &backup, target.path()).await;
    assert!(report.target.is_err());
    backup.recheck().unwrap();
    assert_eq!(tree(target.path()), target_before);
    assert_eq!(tree(source_root.path()), source_before);
    let nested = backup_root.path().join("nested-target");
    fs::create_dir(&nested).unwrap();
    assert!(migrate_verified_v2(&source, &original, &backup, &nested)
        .await
        .target
        .is_err());
    assert!(tree(&nested).is_empty());
}

#[tokio::test]
async fn source_close_after_commit_preserves_known_target_uuid_as_indeterminate() {
    let source_root = tempfile::tempdir().unwrap();
    let backup_root = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    let source = std::sync::Arc::new(fixture(source_root.path()));
    let original = source.capture().unwrap();
    let backup = source
        .backup_committed(&original, backup_root.path())
        .unwrap();
    let close = std::sync::Arc::clone(&source);
    let report = migrate_inner(
        &source,
        &original,
        &backup,
        target.path(),
        Some(Box::new(move || close.close())),
    )
    .await;
    let commit_id = match report.target.unwrap() {
        format::CommitOutcome::Indeterminate { commit_id, .. } => commit_id,
        other => panic!("lost late conflict: {other:?}"),
    };
    let target_session = FolderSession::open(target.path()).await.unwrap();
    let captured = target_session.capture().await.unwrap();
    assert_eq!(captured.commit_id(), commit_id);
    backup.recheck().unwrap();
    assert_eq!(captured.project().id, original.project().id);
}
