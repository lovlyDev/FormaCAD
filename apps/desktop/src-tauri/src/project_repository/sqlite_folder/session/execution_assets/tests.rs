//! Synthetic storage inputs, not CAD geometry fixtures.
use super::*;
use crate::project_repository::sqlite_folder::{creation, format};
use std::{fs, path::Path};

fn descriptor(name: &str, bytes: &[u8]) -> ProjectFile {
    ProjectFile {
        name: name.into(),
        size: bytes.len() as u64,
        kind: "model".into(),
        data: None,
        sha256: Some(crate::artifacts::digest(bytes)),
    }
}

fn project() -> Project {
    let mut project: Project = serde_json::from_value(serde_json::json!({
        "schemaVersion":1,"id":uuid::Uuid::new_v4().to_string(),"name":"execution assets fixture",
        "units":"mm","agent":"codex","pinned":false,"createdAt":"2026-11-01T00:00:00Z",
        "updatedAt":"2026-11-01T00:00:00Z","currentRevision":null,"messages":[],
        "exports":[],"files":[],"revisions":[]
    }))
    .unwrap();
    project.files = vec![
        descriptor("one.step", b"shared synthetic STEP"),
        descriptor("alias.step", b"shared synthetic STEP"),
        descriptor("unused.step", b"unused synthetic STEP"),
    ];
    project
}

async fn fixture(root: &Path) -> FolderSession {
    let project = project();
    let pending = vec![
        (
            project.files[0].sha256.clone().unwrap(),
            b"shared synthetic STEP".to_vec(),
        ),
        (
            project.files[2].sha256.clone().unwrap(),
            b"unused synthetic STEP".to_vec(),
        ),
    ];
    assert!(matches!(
        creation::create(root, project, pending, None)
            .await
            .unwrap(),
        format::CommitOutcome::Committed(_)
    ));
    FolderSession::open(root).await.unwrap()
}

#[tokio::test]
async fn selected_descriptors_share_one_buffer_and_unused_cas_is_not_read() {
    let root = tempfile::tempdir().unwrap();
    let session = fixture(root.path()).await;
    let original = session.capture().await.unwrap();
    let files = &original.project().files;
    fs::remove_file(
        root.path()
            .join("assets")
            .join(files[2].sha256.as_ref().unwrap()),
    )
    .unwrap();
    let mut source = ExecutionAssets::prefetch(&session, &original, &files[..2])
        .await
        .unwrap();
    assert_eq!(source.bytes.len(), 1);
    assert_eq!(source.read(&files[0]).unwrap(), b"shared synthetic STEP");
    assert_eq!(source.read(&files[1]).unwrap(), b"shared synthetic STEP");
    assert!(source.read(&files[2]).is_err());
    let mut forged = files[0].clone();
    forged.name = "not-requested.step".into();
    assert!(source.read(&forged).is_err());
    assert!(ExecutionAssets::prefetch(&session, &original, &[forged])
        .await
        .is_err());
}

#[tokio::test]
async fn same_hash_unrequested_name_and_closed_opening_are_refused() {
    let root = tempfile::tempdir().unwrap();
    let session = fixture(root.path()).await;
    let original = session.capture().await.unwrap();
    let files = &original.project().files;
    let mut source = ExecutionAssets::prefetch(&session, &original, &files[..1])
        .await
        .unwrap();
    assert!(source.read(&files[1]).is_err());
    session.close();
    assert!(source.read(&files[0]).is_err());
    assert!(ExecutionAssets::prefetch(&session, &original, &files[..1])
        .await
        .is_err());
}

#[tokio::test]
async fn copied_folder_opening_cannot_prefetch_with_original_snapshot() {
    let root = tempfile::tempdir().unwrap();
    let copy = tempfile::tempdir().unwrap();
    let session = fixture(root.path()).await;
    let original = session.capture().await.unwrap();
    fs::create_dir(copy.path().join("assets")).unwrap();
    fs::copy(
        root.path().join("project.sqlite"),
        copy.path().join("project.sqlite"),
    )
    .unwrap();
    for entry in fs::read_dir(root.path().join("assets")).unwrap() {
        let entry = entry.unwrap();
        fs::copy(
            entry.path(),
            copy.path().join("assets").join(entry.file_name()),
        )
        .unwrap();
    }
    let other = FolderSession::open(copy.path()).await.unwrap();
    assert!(
        ExecutionAssets::prefetch(&other, &original, &original.project().files[..1])
            .await
            .is_err()
    );
    let fresh = other.capture().await.unwrap();
    let mut source = ExecutionAssets::prefetch(&other, &fresh, &fresh.project().files[..1])
        .await
        .unwrap();
    assert_eq!(
        source.read(&fresh.project().files[0]).unwrap(),
        b"shared synthetic STEP"
    );
}

#[tokio::test]
async fn empty_selection_still_rechecks_original_sql_state() {
    let root = tempfile::tempdir().unwrap();
    let session = fixture(root.path()).await;
    let original = session.capture().await.unwrap();
    let empty = ExecutionAssets::prefetch(&session, &original, &[])
        .await
        .unwrap();
    assert!(empty.bytes.is_empty());
    drop(empty);
    let mut metadata = original.project().clone();
    metadata.name = "new saved metadata".into();
    assert!(matches!(
        session
            .commit(&original, metadata, vec![], None)
            .await
            .unwrap(),
        format::CommitOutcome::Committed(_)
    ));
    assert!(ExecutionAssets::prefetch(&session, &original, &[])
        .await
        .is_err());
}

#[test]
fn descriptor_count_individual_size_and_unique_total_are_checked_without_asset_buffers() {
    let mut project = project();
    let duplicated = vec![project.files[0].clone(); project.files.len() + 1];
    assert!(select(&project, &duplicated).is_err());
    project.files[0].size = crate::artifacts::MAX_FILE_BYTES as u64 + 1;
    assert!(select(&project, &project.files[..1]).is_err());
    project.files = (0..3)
        .map(|index| ProjectFile {
            name: format!("large-{index}.step"),
            size: crate::artifacts::MAX_FILE_BYTES as u64,
            kind: "model".into(),
            data: None,
            sha256: Some(format!("{index:064x}")),
        })
        .collect();
    assert!(select(&project, &project.files).is_err());
    // Repeated names/hash references do not consume repeated retained buffers.
    let selected = select(
        &project,
        &[project.files[0].clone(), project.files[0].clone()],
    )
    .unwrap();
    assert_eq!(selected.len(), 2);
    project.files[1].sha256 = project.files[0].sha256.clone();
    project.files[1].size -= 1;
    assert!(select(&project, &project.files[..2]).is_err());
}
