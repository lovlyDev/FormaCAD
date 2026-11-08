//! Copy/history/source-purity fixtures use synthetic artifacts, not CAD output.
use super::*;
use crate::{
    models::{Project, ProjectFile},
    project_access::folder_lease::FolderLease,
    project_history::Direction,
};
use std::{collections::BTreeMap, fs, path::PathBuf};

fn file(name: &str, bytes: &[u8]) -> ProjectFile {
    ProjectFile {
        name: name.into(),
        size: bytes.len() as u64,
        kind: "model".into(),
        data: None,
        sha256: Some(crate::artifacts::digest(bytes)),
    }
}

fn assert_restored(project: &Project, baseline: &crate::models::Revision, parent: Option<&str>) {
    let current = project
        .revisions
        .iter()
        .find(|revision| Some(&revision.id) == project.current_revision.as_ref())
        .unwrap();
    assert!(crate::project_history::model_context::same_snapshot(
        &baseline.into(),
        &current.into()
    ));
    // History materializes a fresh immutable revision instead of reviving an ID.
    assert_ne!(current.id, baseline.id);
    assert_eq!(current.parent.as_deref(), parent);
}

async fn fixture(root: &Path) -> FolderSession {
    let revision = uuid::Uuid::new_v4().to_string();
    let mut project: Project = serde_json::from_value(serde_json::json!({
        "schemaVersion":1,"id":uuid::Uuid::new_v4().to_string(),"name":"copy fixture",
        "units":"mm","agent":"codex","pinned":false,"createdAt":"2026-11-01T00:00:00Z",
        "updatedAt":"2026-11-01T00:00:00Z","currentRevision":revision,"messages":[],"exports":[],"files":[],
        "revisions":[{"id":revision,"parent":null,"createdAt":"2026-11-01T00:00:00Z","prompt":"synthetic A","source":"a.step",
        "parameters":{"kind":"box","width":40,"depth":20,"height":10,"thickness":2,"holeDiameter":0,"holes":0}}]
    })).unwrap();
    let bytes = b"copy synthetic A";
    project.files.push(file("a.step", bytes));
    project.files.push(file("same-content.step", bytes));
    assert!(matches!(
        creation::create(
            root,
            project,
            vec![(crate::artifacts::digest(bytes), bytes.to_vec())],
            None
        )
        .await
        .unwrap(),
        format::CommitOutcome::Committed(_)
    ));
    let session = FolderSession::open(root).await.unwrap();
    let first = session.capture().await.unwrap();
    let mut next = first.project().clone();
    let mut revision = next.revisions[0].clone();
    revision.id = uuid::Uuid::new_v4().to_string();
    revision.parent = next.current_revision.clone();
    revision.parameters.width = 80.0;
    revision.source = Some("b.step".into());
    next.current_revision = Some(revision.id.clone());
    next.revisions.push(revision);
    let bytes = b"copy synthetic B";
    next.files.push(file("b.step", bytes));
    assert!(matches!(
        session
            .commit(
                &first,
                next,
                vec![(crate::artifacts::digest(bytes), bytes.to_vec())],
                None
            )
            .await
            .unwrap(),
        format::CommitOutcome::Committed(_)
    ));
    let second = session.capture().await.unwrap();
    assert!(matches!(
        session
            .apply_history(&second, Direction::Undo)
            .await
            .unwrap(),
        format::CommitOutcome::Committed(_)
    ));
    session
}

fn tree(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    fn walk(root: &Path, dir: &Path, entries: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let meta = fs::symlink_metadata(&path).unwrap();
            assert!(!meta.file_type().is_symlink());
            if meta.is_dir() {
                entries.insert(path.strip_prefix(root).unwrap().into(), None);
                walk(root, &path, entries);
            } else {
                // Empty OS-locked writer.lock cannot be read on Windows.
                let bytes = if meta.len() == 0 {
                    vec![]
                } else {
                    fs::read(&path).unwrap()
                };
                entries.insert(path.strip_prefix(root).unwrap().into(), Some(bytes));
            }
        }
    }
    let mut entries = BTreeMap::new();
    walk(root, root, &mut entries);
    entries
}

#[tokio::test]
async fn read_only_copy_under_foreign_writer_lease_preserves_source_and_independent_redo() {
    let source_root = tempfile::tempdir().unwrap();
    let destination = tempfile::tempdir().unwrap();
    let source = fixture(source_root.path()).await;
    let original = source.capture().await.unwrap();
    let source_history = source.read_history(&original).await.unwrap();
    let mut reader = super::super::read_only::ReadOnlyDatabase::open(source_root.path())
        .await
        .unwrap();
    let source_saved = format::read(reader.conn()).await.unwrap();
    drop(reader);
    let foreign_writer = FolderLease::acquire(source_root.path()).unwrap();
    let before = tree(source_root.path());
    fn assert_send(_: impl std::future::Future + Send) {}
    // Drop this unpolled future; it proves queue/task compatibility without IO.
    assert_send(save_copy(&source, &original, destination.path()));
    let result = save_copy(&source, &original, destination.path())
        .await
        .unwrap();
    let receipt = match result {
        format::CommitOutcome::Committed(saved) => saved,
        other => panic!("copy not committed: {other:?}"),
    };
    assert_ne!(receipt.project().id, original.project().id);
    assert_ne!(
        receipt.expectation().repository_id(),
        source_saved.expectation().repository_id()
    );
    assert_ne!(
        receipt.expectation().commit_id(),
        source_saved.expectation().commit_id()
    );
    assert_ne!(receipt.expectation().repository_id(), original.project().id);
    assert_ne!(receipt.expectation().repository_id(), receipt.project().id);
    assert_eq!(receipt.project().revisions, original.project().revisions);
    assert_eq!(tree(source_root.path()), before);
    assert_eq!(
        fs::read_dir(destination.path().join("assets"))
            .unwrap()
            .count(),
        2
    );

    let copied = FolderSession::open(destination.path()).await.unwrap();
    let captured = copied.capture().await.unwrap();
    assert_eq!(
        copied.read_history(&captured).await.unwrap(),
        source_history
    );
    assert!(matches!(
        copied
            .apply_history(&captured, Direction::Redo)
            .await
            .unwrap(),
        format::CommitOutcome::Committed(_)
    ));
    let redone = copied.capture().await.unwrap();
    assert_restored(
        redone.project(),
        &original.project().revisions[1],
        original.project().current_revision.as_deref(),
    );
    assert_eq!(
        copied
            .read_asset(&redone, &redone.project().files[2])
            .await
            .unwrap(),
        b"copy synthetic B"
    );
    assert!(matches!(
        copied
            .apply_history(&redone, Direction::Undo)
            .await
            .unwrap(),
        format::CommitOutcome::Committed(_)
    ));
    let undone = copied.capture().await.unwrap();
    assert_restored(
        undone.project(),
        &original.project().revisions[0],
        redone.project().current_revision.as_deref(),
    );
    assert!(matches!(
        copied
            .apply_history(&undone, Direction::Redo)
            .await
            .unwrap(),
        format::CommitOutcome::Committed(_)
    ));
    let redone_again = copied.capture().await.unwrap();
    assert_restored(
        redone_again.project(),
        &original.project().revisions[1],
        undone.project().current_revision.as_deref(),
    );
    assert!(!redone_again.history_state().can_redo());
    source.recheck(&original).await.unwrap();
    assert_eq!(
        source.read_history(&original).await.unwrap(),
        source_history
    );
    assert_eq!(tree(source_root.path()), before);
    drop(foreign_writer);
}

#[tokio::test]
async fn stale_closed_and_other_opening_snapshots_do_not_touch_destination() {
    let source_root = tempfile::tempdir().unwrap();
    let source = fixture(source_root.path()).await;
    let original = source.capture().await.unwrap();
    let other = FolderSession::open(source_root.path()).await.unwrap();
    let wrong_destination = tempfile::tempdir().unwrap();
    assert!(save_copy(&other, &original, wrong_destination.path())
        .await
        .is_err());
    assert!(tree(wrong_destination.path()).is_empty());
    let mut next = original.project().clone();
    next.name = "new metadata".into();
    assert!(matches!(
        source.commit(&original, next, vec![], None).await.unwrap(),
        format::CommitOutcome::Committed(_)
    ));
    let stale_destination = tempfile::tempdir().unwrap();
    assert!(save_copy(&source, &original, stale_destination.path())
        .await
        .is_err());
    assert!(tree(stale_destination.path()).is_empty());
    let current = source.capture().await.unwrap();
    source.close();
    let closed_destination = tempfile::tempdir().unwrap();
    assert!(save_copy(&source, &current, closed_destination.path())
        .await
        .is_err());
    assert!(tree(closed_destination.path()).is_empty());
}

#[tokio::test]
async fn nonempty_destination_and_same_source_are_never_overwritten() {
    let source_root = tempfile::tempdir().unwrap();
    let source = fixture(source_root.path()).await;
    let original = source.capture().await.unwrap();
    let destination = tempfile::tempdir().unwrap();
    fs::write(destination.path().join("keep.txt"), b"existing destination").unwrap();
    let before_source = tree(source_root.path());
    let before_destination = tree(destination.path());
    assert!(save_copy(&source, &original, destination.path())
        .await
        .is_err());
    assert_eq!(tree(destination.path()), before_destination);
    assert!(save_copy(&source, &original, source_root.path())
        .await
        .is_err());
    assert_eq!(tree(source_root.path()), before_source);
}

#[tokio::test]
async fn creation_boundary_rechecks_closed_source_after_cas_before_any_sql_publication() {
    let source_root = tempfile::tempdir().unwrap();
    let source = fixture(source_root.path()).await;
    let original = source.capture().await.unwrap();
    let destination = tempfile::tempdir().unwrap();
    let mut candidate = original.project().clone();
    candidate.id = uuid::Uuid::new_v4().to_string();
    let mut pending = Vec::new();
    let mut seen = HashSet::new();
    for file in &original.project().files {
        let hash = file.sha256.as_ref().unwrap();
        if seen.insert(hash) {
            pending.push((
                hash.clone(),
                source.read_asset(&original, file).await.unwrap(),
            ));
        }
    }
    let history = source.read_history(&original).await.unwrap();
    let result = creation::create_checked(
        destination.path(),
        candidate,
        pending,
        Some(history),
        || async {
            assert_eq!(
                fs::read_dir(destination.path().join("assets"))
                    .unwrap()
                    .count(),
                2
            );
            assert!(!destination.path().join("project.sqlite").exists());
            source.close();
            source.recheck(&original).await
        },
    )
    .await;
    assert!(result.is_err());
    assert!(!destination.path().join("project.sqlite").exists());
    // Own staged CAS may remain for explicit recovery. Nothing was SQL-published.
    assert_eq!(
        fs::read_dir(destination.path().join("assets"))
            .unwrap()
            .count(),
        2
    );
}
