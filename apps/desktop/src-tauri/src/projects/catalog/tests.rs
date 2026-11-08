//! Exercise host helpers with real SQL without recovery or native CAD fixtures.
use super::*;
use crate::project_repository::{catalog::CatalogEntry, legacy::read_test_fixture as fixture};

#[tokio::test]
async fn mixed_catalog_and_target_read_never_recover_or_hide_pending_projects() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let healthy = fixture::project(&state, 16).await;
    let pending = fixture::project(&state, 16).await;
    fixture::pending(&state, &pending.id).await;
    let healthy_before = fixture::logical(&state, &healthy.id).await;
    let pending_before = fixture::logical(&state, &pending.id).await;
    let files_before = fixture::files(temp.path());
    let report = list(&state).await.unwrap();
    assert_eq!(report.entries.len(), 2);
    assert!(report.entries.iter().any(
        |entry| matches!(entry, CatalogEntry::Ready { project, .. } if project.id == healthy.id)
    ));
    assert!(report.entries.iter().any(|entry| matches!(entry, CatalogEntry::Unavailable { index, .. } if index.project_id.as_deref() == Some(pending.id.as_str()))));
    assert_eq!(read(&state, &healthy.id).await.unwrap().id, healthy.id);
    assert_eq!(
        read(&state, &pending.id).await.err().unwrap().to_string(),
        "PROJECT_SNAPSHOT_RECOVERY_REQUIRED"
    );
    assert_eq!(fixture::logical(&state, &healthy.id).await, healthy_before);
    assert_eq!(fixture::logical(&state, &pending.id).await, pending_before);
    assert_eq!(fixture::files(temp.path()), files_before);
    assert_eq!(
        state
            .project_access
            .status(&state.root, &healthy.id)
            .unwrap()
            .mode,
        "closed"
    );
    assert!(!state.root.join(&healthy.id).exists());
    assert!(!state.root.join(&pending.id).exists());
}
