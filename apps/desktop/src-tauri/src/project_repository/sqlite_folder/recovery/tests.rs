use super::*;
use crate::{
    models::{Project, ProjectFile},
    project_repository::sqlite_folder::api::{FolderProject, Publication},
};
use std::process::Command;

const ASSET: &[u8] = b"sealed FCP1 attachment before an interrupted operation";
const ORIGINAL: &str = "Committed recovery fixture";

async fn pending_source(root: &Path) -> String {
    let mut project: Project = serde_json::from_value(serde_json::json!({
        "schemaVersion":1,"id":uuid::Uuid::new_v4().to_string(),"name":ORIGINAL,
        "units":"mm","agent":"codex","pinned":false,"createdAt":"2026-11-01T00:00:00Z",
        "updatedAt":"2026-11-01T00:00:00Z","currentRevision":null,"messages":[],
        "exports":[],"files":[],"revisions":[]
    }))
    .unwrap();
    let hash = crate::artifacts::digest(ASSET);
    project.files.push(ProjectFile {
        name: "original.step".into(),
        kind: "model".into(),
        size: ASSET.len() as u64,
        data: None,
        sha256: Some(hash.clone()),
    });
    let Publication::Committed(saved) =
        FolderProject::create(root, project, vec![(hash, ASSET.to_vec())], None)
            .await
            .unwrap()
    else {
        panic!("fixture must be committed before abrupt exit");
    };
    let before = fs::read(root.join("project.sqlite")).unwrap();
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "project_repository::sqlite_folder::write_connection::tests::hot_journal_child",
            "--nocapture",
        ])
        .env("FORMA_FCP1_WRITER_CRASH_ROOT", root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let journal = fs::read(root.join("project.sqlite-journal")).unwrap();
    assert!(journal.starts_with(&[0xd9, 0xd5, 0x05, 0xf9, 0x20, 0xa1, 0x63, 0xd7]));
    assert_ne!(before, fs::read(root.join("project.sqlite")).unwrap());
    saved.project().id.clone()
}

#[tokio::test]
async fn explicit_recovery_replays_a_real_hot_journal_only_in_private_scratch() {
    let root = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    let scratch = tempfile::tempdir().unwrap();
    let project_id = pending_source(root.path()).await;
    let database = root.path().join("project.sqlite");
    let journal = root.path().join("project.sqlite-journal");
    let before = (fs::read(&database).unwrap(), fs::read(&journal).unwrap());
    assert!(matches!(
        FolderProject::open(root.path()).await,
        Err(AppError::Invalid(reason)) if reason == "PROJECT_SNAPSHOT_RECOVERY_REQUIRED"
    ));

    let Publication::Committed(copy) =
        FolderProject::recover_to_copy(root.path(), target.path(), scratch.path())
            .await
            .unwrap()
    else {
        panic!("recovery must publish a verified independent copy");
    };
    assert_ne!(copy.project().id, project_id);
    let reopened = FolderProject::open(target.path()).await.unwrap();
    let view = reopened.capture().await.unwrap();
    assert_eq!(view.project().name, ORIGINAL);
    assert_eq!(view.commit_id(), copy.commit_id());
    let history = reopened.read_history(&view).await.unwrap();
    crate::project_history::validate_archive(view.project(), &history).unwrap();
    assert_eq!(
        reopened
            .read_asset(&view, &view.project().files[0])
            .await
            .unwrap(),
        ASSET
    );
    assert_eq!(fs::read(database).unwrap(), before.0);
    assert_eq!(fs::read(journal).unwrap(), before.1);
    assert_eq!(
        fs::read_dir(scratch.path().join(".transient"))
            .unwrap()
            .count(),
        0
    );
}

#[tokio::test]
async fn recovery_refuses_a_source_changed_to_a_newer_commit_after_capture() {
    let root = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    let scratch = tempfile::tempdir().unwrap();
    let project_id = pending_source(root.path()).await;
    let (capture, workspace) = stage(root.path(), scratch.path()).unwrap();

    // Deliberately emulate an external SQLite writer that does not take the
    // cooperative .forma lease. Its published head must survive stale recovery.
    let db = root.path().join("project.sqlite");
    let mut external = SqliteConnection::connect_with(
        &SqliteConnectOptions::new()
            .filename(&db)
            .create_if_missing(false)
            .pragma("foreign_keys", "ON"),
    )
    .await
    .unwrap();
    let previous = format::read(&mut external).await.unwrap();
    let mut candidate = previous.project().clone();
    candidate.name = "Newer external commit".into();
    let format::CommitOutcome::Committed(newer) =
        format::commit(&mut external, &previous, &candidate, None)
            .await
            .unwrap()
    else {
        panic!("external writer must publish the new head");
    };
    external.close().await.unwrap();
    let newer_database = fs::read(&db).unwrap();
    let destination_location =
        Arc::new(LocationIdentity::capture(target.path(), Path::new("")).unwrap());
    assert!(matches!(
        recover_captured(capture, workspace, target.path(), destination_location).await,
        Err(AppError::Invalid(reason)) if reason == "PROJECT_SNAPSHOT_CHANGED"
    ));
    assert_eq!(fs::read(db).unwrap(), newer_database);
    assert_eq!(fs::read_dir(target.path()).unwrap().count(), 0);
    let reopened = FolderProject::open(root.path()).await.unwrap();
    let view = reopened.capture().await.unwrap();
    assert_eq!(view.project().id, project_id);
    assert_eq!(view.project().name, "Newer external commit");
    assert_eq!(view.commit_id(), newer.expectation().commit_id());
}

#[tokio::test]
async fn recovery_rejects_journals_with_external_super_journal_names() {
    let root = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    let scratch = tempfile::tempdir().unwrap();
    pending_source(root.path()).await;
    let journal = root.path().join("project.sqlite-journal");
    let mut bytes = fs::read(&journal).unwrap();
    let name = b"/outside/super-journal";
    bytes.extend_from_slice(name);
    bytes.extend_from_slice(&(name.len() as u32).to_be_bytes());
    let checksum: u32 = name.iter().map(|byte| u32::from(*byte)).sum();
    bytes.extend_from_slice(&checksum.to_be_bytes());
    bytes.extend_from_slice(&[0xd9, 0xd5, 0x05, 0xf9, 0x20, 0xa1, 0x63, 0xd7]);
    fs::write(&journal, &bytes).unwrap();
    assert!(
        FolderProject::recover_to_copy(root.path(), target.path(), scratch.path())
            .await
            .is_err()
    );
    assert_eq!(fs::read(journal).unwrap(), bytes);
    assert_eq!(fs::read_dir(target.path()).unwrap().count(), 0);
}
