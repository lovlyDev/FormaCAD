//! Real filesystem/SQLite fixtures. Synthetic bytes assert storage behavior,
//! never CAD validity or physical power-loss durability.
use super::*;
use crate::models::ProjectFile;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode},
    Connection, Executor, SqliteConnection,
};

fn candidate(entries: &[(&str, &[u8])]) -> Project {
    let mut project: Project = serde_json::from_value(serde_json::json!({
        "schemaVersion":1,"id":uuid::Uuid::new_v4().to_string(),"name":"CAS fixture",
        "units":"mm","agent":"codex","pinned":false,"createdAt":"2026-11-01T00:00:00Z",
        "updatedAt":"2026-11-01T00:00:00Z","currentRevision":null,"messages":[],
        "exports":[],"files":[],"revisions":[]
    }))
    .unwrap();
    project.files = entries
        .iter()
        .map(|(name, bytes)| ProjectFile {
            name: (*name).into(),
            size: bytes.len() as u64,
            kind: "model".into(),
            data: None,
            sha256: Some(crate::artifacts::digest(bytes)),
        })
        .collect();
    project
}
fn pending(bytes: &[u8]) -> (String, Vec<u8>) {
    (crate::artifacts::digest(bytes), bytes.to_vec())
}

async fn fixture(root: &Path) -> (FolderLease, ReadOnlyDatabase) {
    fs::create_dir(root.join("assets")).unwrap();
    let lease = FolderLease::acquire(root).unwrap();
    let options = SqliteConnectOptions::new()
        .filename(root.join("project.sqlite"))
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Delete);
    let mut writer = SqliteConnection::connect_with(&options).await.unwrap();
    writer
        .execute("CREATE TABLE saved(value INTEGER); INSERT INTO saved VALUES(7)")
        .await
        .unwrap();
    writer.close().await.unwrap();
    let reader = ReadOnlyDatabase::open(root).await.unwrap();
    (lease, reader)
}

#[tokio::test]
async fn duplicate_pending_bytes_publish_one_entry_and_existing_reuse_changes_nothing() {
    let root = tempfile::tempdir().unwrap();
    let (lease, mut reader) = fixture(root.path()).await;
    let bytes = b"same synthetic bytes";
    let project = candidate(&[("first.step", bytes), ("second.step", bytes)]);
    let assets = vec![pending(bytes), pending(bytes)];
    let published = PreparedCasBatch::validate(&project, &assets)
        .unwrap()
        .publish(&lease, &mut reader)
        .unwrap();
    published.recheck(&mut reader).unwrap();
    assert_eq!(fs::read_dir(root.path().join("assets")).unwrap().count(), 1);
    assert_eq!(
        reader.read_blob(&assets[0].0, bytes.len() as u64).unwrap(),
        bytes
    );
    // Retained newly-created handles also keep the writer authority alive.
    drop(lease);
    assert!(FolderLease::acquire(root.path()).is_err());
    drop(published);
    let lease = FolderLease::acquire(root.path()).unwrap();
    let reused = PreparedCasBatch::validate(&project, &[])
        .unwrap()
        .publish(&lease, &mut reader)
        .unwrap();
    reused.recheck(&mut reader).unwrap();
    assert_eq!(fs::read_dir(root.path().join("assets")).unwrap().count(), 1);
    reader.close().await.unwrap();
}

#[tokio::test]
async fn corrupt_existing_blob_refuses_entire_batch_without_overwriting_or_creating_others() {
    let root = tempfile::tempdir().unwrap();
    let (lease, mut reader) = fixture(root.path()).await;
    let bad = b"expected existing bytes";
    let new = b"new candidate bytes";
    let project = candidate(&[("old.step", bad), ("new.glb", new)]);
    let hash = crate::artifacts::digest(bad);
    let path = root.path().join("assets").join(&hash);
    let corrupt = vec![b'X'; bad.len()];
    fs::write(&path, &corrupt).unwrap();
    let assets = vec![pending(bad), pending(new)];
    assert!(PreparedCasBatch::validate(&project, &assets)
        .unwrap()
        .publish(&lease, &mut reader)
        .is_err());
    assert_eq!(fs::read(&path).unwrap(), corrupt);
    assert_eq!(fs::read_dir(root.path().join("assets")).unwrap().count(), 1);
    reader.recheck().await.unwrap();
    reader.close().await.unwrap();
}

#[test]
fn every_pending_entry_is_validated_before_publication_and_unused_entries_are_rejected() {
    let root = tempfile::tempdir().unwrap();
    let lease = FolderLease::initialize_empty(root.path()).unwrap();
    let bytes = b"valid synthetic bytes";
    let project = candidate(&[("first.step", bytes)]);
    let mut invalid = pending(bytes);
    invalid.1[0] ^= 1;
    for batch in [
        vec![pending(bytes), invalid],
        vec![pending(bytes), pending(b"unused")],
    ] {
        assert!(PreparedCasBatch::validate(&project, &batch).is_err());
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1); // only .forma
        assert!(!root.path().join("assets").exists());
    }
    lease.recheck().unwrap();
}

#[test]
fn bootstrap_requires_all_bytes_and_never_accepts_an_existing_digest() {
    let root = tempfile::tempdir().unwrap();
    let lease = FolderLease::initialize_empty(root.path()).unwrap();
    let bytes = b"bootstrap fixture bytes";
    let project = candidate(&[("first.step", bytes)]);
    assert!(PreparedCasBatch::validate(&project, &[])
        .unwrap()
        .publish_new(&lease)
        .is_err());
    assert!(!root.path().join("assets").exists());
    let assets = vec![pending(bytes)];
    let published = PreparedCasBatch::validate(&project, &assets)
        .unwrap()
        .publish_new(&lease)
        .unwrap();
    published.recheck_new().unwrap();
    assert!(!root.path().join("project.sqlite").exists());
    drop(published);
    assert!(PreparedCasBatch::validate(&project, &assets)
        .unwrap()
        .publish_new(&lease)
        .is_err());
    assert_eq!(
        fs::read(root.path().join("assets").join(&assets[0].0)).unwrap(),
        bytes
    );
}

#[tokio::test]
async fn partial_unreferenced_entry_is_preserved_and_not_silently_repaired() {
    let root = tempfile::tempdir().unwrap();
    let (lease, mut reader) = fixture(root.path()).await;
    let bytes = b"complete intended synthetic bytes";
    let assets = vec![pending(bytes)];
    let project = candidate(&[("fixture.step", bytes)]);
    let directory = lease.assets_directory().unwrap();
    let mut interrupted = platform::NewFile::create(directory.path(), &assets[0].0).unwrap();
    interrupted.write(&bytes[..5]).unwrap();
    drop(interrupted); // models process interruption before the rest was written
    assert!(PreparedCasBatch::validate(&project, &assets)
        .unwrap()
        .publish(&lease, &mut reader)
        .is_err());
    assert_eq!(
        fs::read(root.path().join("assets").join(&assets[0].0)).unwrap(),
        &bytes[..5]
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT value FROM saved")
            .fetch_one(reader.conn())
            .await
            .unwrap(),
        7
    );
    reader.close().await.unwrap();
}

#[tokio::test]
async fn hardlinked_existing_blob_is_refused_without_touching_the_other_name() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let (lease, mut reader) = fixture(root.path()).await;
    let bytes = b"hardlink fixture bytes";
    let assets = vec![pending(bytes)];
    let project = candidate(&[("fixture.step", bytes)]);
    let original = outside.path().join("original");
    fs::write(&original, bytes).unwrap();
    fs::hard_link(&original, root.path().join("assets").join(&assets[0].0)).unwrap();
    assert!(PreparedCasBatch::validate(&project, &assets)
        .unwrap()
        .publish(&lease, &mut reader)
        .is_err());
    assert_eq!(fs::read(&original).unwrap(), bytes);
    assert_eq!(fs::read_dir(root.path().join("assets")).unwrap().count(), 1);
    reader.close().await.unwrap();
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn linked_or_replaced_assets_directory_receives_no_publication() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let (lease, mut reader) = fixture(root.path()).await;
    let bytes = b"source replacement fixture";
    let project = candidate(&[("fixture.step", bytes)]);
    let assets = vec![pending(bytes)];
    fs::rename(root.path().join("assets"), root.path().join("old-assets")).unwrap();
    std::os::unix::fs::symlink(outside.path(), root.path().join("assets")).unwrap();
    assert!(PreparedCasBatch::validate(&project, &assets)
        .unwrap()
        .publish(&lease, &mut reader)
        .is_err());
    assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 0);
    assert_eq!(
        fs::read_dir(root.path().join("old-assets"))
            .unwrap()
            .count(),
        0
    );
    reader.close().await.unwrap();
}

#[cfg(target_os = "linux")]
#[test]
fn retained_new_file_detects_content_and_name_replacement() {
    let root = tempfile::tempdir().unwrap();
    let lease = FolderLease::initialize_empty(root.path()).unwrap();
    let bytes = b"owned handle fixture";
    let project = candidate(&[("fixture.step", bytes)]);
    let assets = vec![pending(bytes)];
    let published = PreparedCasBatch::validate(&project, &assets)
        .unwrap()
        .publish_new(&lease)
        .unwrap();
    let path = root.path().join("assets").join(&assets[0].0);
    fs::write(&path, vec![b'X'; bytes.len()]).unwrap();
    assert!(published.recheck_new().is_err());
    fs::rename(&path, root.path().join("assets/replaced")).unwrap();
    fs::write(&path, bytes).unwrap();
    assert!(published.recheck_new().is_err());
}
