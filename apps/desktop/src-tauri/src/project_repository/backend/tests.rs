use super::ProjectRepository;
use crate::{models::Project, project_repository::legacy::read_test_fixture as fixture};
use std::{future::Future, path::Path, task::Poll};

fn code<T>(result: crate::core::Result<T>) -> String {
    result
        .err()
        .expect("Expected rejected repository read")
        .to_string()
}

fn folder(root: &Path, project: &Project, history: Option<&[u8]>) {
    let mut project = project.clone();
    let pending = crate::artifacts::normalize(&mut project).unwrap();
    let mut staged = crate::project_storage_v2::stage_with(root, &project, |file| {
        Ok(pending
            .iter()
            .find(|(hash, _)| Some(hash) == file.sha256.as_ref())
            .unwrap()
            .1
            .clone())
    })
    .unwrap();
    if let Some(history) = history {
        crate::project_history::validate_archive(&project, history).unwrap();
        crate::project_storage_v2::stage_history(root, &mut staged, history).unwrap();
    }
    crate::project_storage_v2::publish(root, staged).unwrap();
}

#[tokio::test]
async fn managed_dispatch_reads_bound_assets_and_explicit_archive_without_writer_access() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let project = fixture::project(&state, 16).await;
    let before = fixture::logical(&state, &project.id).await;
    let files = fixture::files(temp.path());
    let repository = ProjectRepository::managed(&state, &project.id).unwrap();
    let captured = repository.capture().await.unwrap();
    assert_eq!(captured.project().id, project.id);
    assert_eq!(
        repository
            .read_asset(&captured, &captured.project().files[0])
            .await
            .unwrap(),
        vec![b'x'; 16]
    );
    assert_eq!(
        code(repository.read_history(&captured).await),
        "HISTORY_INVALID_STATE"
    );
    let archive_snapshot = repository.capture_with_history().await.unwrap();
    let archive = repository
        .read_history(&archive_snapshot)
        .await
        .unwrap()
        .unwrap();
    crate::project_history::validate_archive(archive_snapshot.project(), &archive).unwrap();
    assert_eq!(
        state
            .project_access
            .status(&state.root, &project.id)
            .unwrap()
            .mode,
        "closed"
    );
    assert_eq!(fixture::logical(&state, &project.id).await, before);
    assert_eq!(fixture::files(temp.path()), files);
}

#[tokio::test]
async fn managed_close_rejects_original_metadata_and_archive_without_refreshing_authority() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let project = fixture::project(&state, 16).await;
    let repository = ProjectRepository::managed(&state, &project.id).unwrap();
    let metadata = repository.capture().await.unwrap();
    let archive = repository.capture_with_history().await.unwrap();
    let before = fixture::logical(&state, &project.id).await;
    let files = fixture::files(temp.path());
    state.project_access.release(&project.id).unwrap();
    assert_eq!(
        code(repository.recheck(&metadata).await),
        "PROJECT_SNAPSHOT_CHANGED"
    );
    assert_eq!(
        code(
            repository
                .read_asset(&metadata, &metadata.project().files[0])
                .await
        ),
        "PROJECT_SNAPSHOT_CHANGED"
    );
    assert_eq!(
        code(repository.read_history(&archive).await),
        "PROJECT_SNAPSHOT_CHANGED"
    );
    let fresh = repository.capture().await.unwrap();
    repository.recheck(&fresh).await.unwrap();
    assert_eq!(fixture::logical(&state, &project.id).await, before);
    assert_eq!(fixture::files(temp.path()), files);
}

#[tokio::test]
async fn identical_ids_in_two_managed_roots_cannot_exchange_snapshots() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let state = fixture::state(first.path()).await;
    let other_state = fixture::state(second.path()).await;
    let project = fixture::project(&state, 16).await;
    let mut other = project.clone();
    other.files[0].data = Some(crate::artifacts::data_url("model.step", &[b'y'; 16]));
    other.files[0].sha256 = Some(crate::artifacts::digest(&[b'y'; 16]));
    fixture::replace(&other_state, &other, true).await;
    let left = ProjectRepository::managed(&state, &project.id).unwrap();
    let right = ProjectRepository::managed(&other_state, &project.id).unwrap();
    let left_snapshot = left.capture().await.unwrap();
    let right_snapshot = right.capture().await.unwrap();
    assert_eq!(left_snapshot.project().id, right_snapshot.project().id);
    assert_eq!(
        left_snapshot.project().current_revision,
        right_snapshot.project().current_revision
    );
    assert_eq!(
        code(right.recheck(&left_snapshot).await),
        "PROJECT_SNAPSHOT_CHANGED"
    );
    assert_eq!(
        code(left.recheck(&right_snapshot).await),
        "PROJECT_SNAPSHOT_CHANGED"
    );
    assert_eq!(
        left.read_asset(&left_snapshot, &left_snapshot.project().files[0])
            .await
            .unwrap(),
        vec![b'x'; 16]
    );
    assert_eq!(
        right
            .read_asset(&right_snapshot, &right_snapshot.project().files[0])
            .await
            .unwrap(),
        vec![b'y'; 16]
    );
}

#[tokio::test]
async fn independent_folder_and_managed_backend_reject_each_others_authority() {
    let managed_root = tempfile::tempdir().unwrap();
    let external_root = tempfile::tempdir().unwrap();
    let state = fixture::state(managed_root.path()).await;
    let project = fixture::project(&state, 16).await;
    let managed = ProjectRepository::managed(&state, &project.id).unwrap();
    let managed_snapshot = managed.capture_with_history().await.unwrap();
    let archive = managed
        .read_history(&managed_snapshot)
        .await
        .unwrap()
        .unwrap();
    folder(external_root.path(), &project, Some(&archive));
    let before = fixture::files(external_root.path());
    let external = ProjectRepository::folder(external_root.path()).unwrap();
    let external_snapshot = external.capture().await.unwrap();
    assert_eq!(external_snapshot.project().id, project.id);
    assert_eq!(
        code(managed.recheck(&external_snapshot).await),
        "PROJECT_SNAPSHOT_CHANGED"
    );
    assert_eq!(
        code(external.recheck(&managed_snapshot).await),
        "PROJECT_SNAPSHOT_CHANGED"
    );
    assert_eq!(
        external
            .read_asset(&external_snapshot, &external_snapshot.project().files[0])
            .await
            .unwrap(),
        vec![b'x'; 16]
    );
    assert_eq!(
        external.read_history(&external_snapshot).await.unwrap(),
        Some(archive)
    );
    assert_eq!(fixture::files(external_root.path()), before);
}

#[tokio::test]
async fn asset_descriptor_must_match_captured_membership_and_embedded_seal() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let mut project = fixture::project(&state, 16).await;
    let repository = ProjectRepository::managed(&state, &project.id).unwrap();
    let captured = repository.capture().await.unwrap();
    let mut forged = captured.project().files[0].clone();
    forged.name = "another.step".into();
    assert!(code(repository.read_asset(&captured, &forged).await).contains("integrity"));

    // Legacy embedded data must match its saved size/checksum as well: the
    // base artifact decoder alone only checks its maximum decoded size.
    project.files[0].data = Some(crate::artifacts::data_url("model.step", &[b'y'; 16]));
    fixture::replace(&state, &project, false).await;
    let changed = repository.capture().await.unwrap();
    assert!(code(
        repository
            .read_asset(&changed, &changed.project().files[0])
            .await
    )
    .contains("integrity"));
}

#[tokio::test]
async fn close_during_asset_sql_recheck_rejects_dispatch_without_mutating_source() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let project = fixture::project(&state, 16).await;
    let repository = ProjectRepository::managed(&state, &project.id).unwrap();
    let captured = repository.capture().await.unwrap();
    let before = fixture::logical(&state, &project.id).await;
    let files = fixture::files(temp.path());
    let mut held = Vec::new();
    for _ in 0..state.pool.options().get_max_connections() {
        held.push(state.pool.acquire().await.unwrap());
    }
    let reading = repository.read_asset(&captured, &captured.project().files[0]);
    tokio::pin!(reading);
    std::future::poll_fn(|context| {
        assert!(reading.as_mut().poll(context).is_pending());
        Poll::Ready(())
    })
    .await;
    state.project_access.release(&project.id).unwrap();
    drop(held);
    assert_eq!(code(reading.await), "PROJECT_SNAPSHOT_CHANGED");
    assert_eq!(fixture::logical(&state, &project.id).await, before);
    assert_eq!(fixture::files(temp.path()), files);
}
