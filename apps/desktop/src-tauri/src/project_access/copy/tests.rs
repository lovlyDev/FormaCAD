//! Actual copy consumer regressions over materialized saved-source fixtures.
//! SQL waits are controlled by held connections, not timing or global hooks.
use super::copy_for_editing;
use crate::{
    core::AppState, models::Project, project_repository::legacy::read_test_fixture as fixture,
};
use std::{
    collections::BTreeMap,
    future::Future,
    path::{Path, PathBuf},
    task::Poll,
};

type EventRows = Vec<(
    i64,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
)>;

#[derive(Debug, PartialEq)]
struct Stored {
    logical: (String, String, i64, i64),
    project_ids: Vec<String>,
    events: EventRows,
    journal: Option<(Option<String>, String, String)>,
    tree: BTreeMap<PathBuf, Option<Vec<u8>>>,
}

fn source_tree(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    fn walk(root: &Path, at: &Path, tree: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
        for entry in std::fs::read_dir(at).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let relative = path.strip_prefix(root).unwrap();
            // The app DB has its own logical snapshot below. Registry OS-lock
            // bytes and informational owner hints change on explicit reopen and
            // are not saved model/history/artifact data. Skip before opening:
            // Windows correctly denies reading an exclusively locked lock file.
            if at == root
                && (entry.file_name() == ".project-access"
                    || entry
                        .file_name()
                        .to_string_lossy()
                        .starts_with("read.sqlite"))
            {
                continue;
            }
            let metadata = std::fs::symlink_metadata(&path).unwrap();
            assert!(!metadata.file_type().is_symlink());
            if metadata.is_dir() {
                // Empty destination directories are observable too.
                tree.insert(relative.into(), None);
                walk(root, &path, tree);
            } else {
                assert!(metadata.is_file());
                tree.insert(relative.into(), Some(std::fs::read(path).unwrap()));
            }
        }
    }
    let mut tree = BTreeMap::new();
    walk(root, root, &mut tree);
    tree
}

async fn stored(state: &AppState, project_id: &str) -> Stored {
    Stored {
        logical: fixture::logical(state, project_id).await,
        project_ids: sqlx::query_scalar("SELECT id FROM projects ORDER BY id")
            .fetch_all(&state.pool).await.unwrap(),
        events: sqlx::query_as("SELECT id,kind,from_revision,to_revision,target_revision,created_at FROM project_history_events WHERE project_id=? ORDER BY id")
            .bind(project_id).fetch_all(&state.pool).await.unwrap(),
        journal: sqlx::query_as("SELECT base_manifest_sha256,target_manifest,project_sha256 FROM project_storage_commits WHERE project_id=?")
            .bind(project_id).fetch_optional(&state.pool).await.unwrap(),
        tree: source_tree(&state.root),
    }
}

async fn materialized(state: &AppState) -> Project {
    let mut project = fixture::project(state, 64).await;
    let pending = crate::artifacts::normalize(&mut project).unwrap();
    let project = crate::projects::persist(state, project, pending, None)
        .await
        .unwrap();
    assert!(project.files.iter().all(|file| file.data.is_none()));
    let root = state.root.join(&project.id);
    assert!(root.join("manifest.json").is_file());
    assert!(root.join("assets").is_dir());
    assert!(root.join("attachments").is_dir());
    project
}

#[tokio::test]
async fn close_and_reopen_during_copy_sql_capture_rejects_without_creating_a_destination() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let project = materialized(&state).await;
    assert_eq!(
        state
            .project_access
            .acquire(&state.root, &project.id)
            .unwrap()
            .mode,
        "write"
    );
    let before = stored(&state, &project.id).await;
    assert_eq!(before.project_ids, vec![project.id.clone()]);
    let mut held = Vec::new();
    for _ in 0..state.pool.options().get_max_connections() {
        held.push(state.pool.acquire().await.unwrap());
    }
    let copying = copy_for_editing(&state, &project.id);
    tokio::pin!(copying);
    std::future::poll_fn(|context| {
        // The actual consumer has captured its read session and cannot get the
        // first SQL connection; no destination staging or timing assumption.
        assert!(copying.as_mut().poll(context).is_pending());
        Poll::Ready(())
    })
    .await;
    state.project_access.release(&project.id).unwrap();
    assert_eq!(
        state
            .project_access
            .acquire(&state.root, &project.id)
            .unwrap()
            .mode,
        "write"
    );
    assert_eq!(source_tree(temp.path()), before.tree);
    drop(held);
    assert_eq!(
        copying.await.err().unwrap().to_string(),
        "PROJECT_SNAPSHOT_CHANGED"
    );
    assert_eq!(stored(&state, &project.id).await, before);
}

#[tokio::test]
async fn pending_recovery_copy_refuses_without_publishing_or_clearing_source_journal() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let project = materialized(&state).await;
    fixture::pending(&state, &project.id).await;
    let before = stored(&state, &project.id).await;
    assert!(before.journal.is_some());
    assert_eq!(before.project_ids, vec![project.id.clone()]);
    assert_eq!(
        copy_for_editing(&state, &project.id)
            .await
            .err()
            .unwrap()
            .to_string(),
        "PROJECT_SNAPSHOT_RECOVERY_REQUIRED"
    );
    assert_eq!(stored(&state, &project.id).await, before);
}
