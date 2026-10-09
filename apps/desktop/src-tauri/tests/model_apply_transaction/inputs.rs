use super::fixture::*;
use forma_core::{modeling::apply::apply_with_executable, models::ProjectFile};
use serde_json::json;

#[tokio::test]
async fn queued_imported_input_corruption_cannot_launch_worker_or_commit() {
    let root = tempfile::tempdir().unwrap();
    let (state, mut baseline) = fixture(root.path()).await;
    let revision = baseline.revisions.last().unwrap();
    let file = baseline
        .files
        .iter()
        .find(|file| Some(&file.name) == revision.source.as_ref())
        .unwrap();
    let mut bytes = forma_core::artifacts::read(&state, &baseline.id, file).unwrap();
    bytes.extend_from_slice(b"\n ");
    let hash = forma_core::artifacts::digest(&bytes);
    baseline.files.push(ProjectFile {
        name: "input.step".into(),
        kind: "model".into(),
        size: bytes.len() as u64,
        sha256: Some(hash.clone()),
        data: Some(forma_core::artifacts::data_url("input.step", &bytes)),
    });
    let pending = forma_core::artifacts::normalize(&mut baseline).unwrap();
    let baseline = forma_core::projects::persist(&state, baseline, pending, None)
        .await
        .unwrap();
    let mut candidate = document(50.);
    candidate["features"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"input","name":"Input","operation":{
        "type":"importStep","assetId":format!("step_{hash}"),"sha256":hash}}));
    let hold = state
        .cad_tasks
        .acquire(&baseline.id, "fixture", token())
        .await
        .unwrap();
    let request = request(&baseline, candidate);
    let task_state = state.clone();
    let missing = root.path().join("must-not-spawn.exe");
    let job =
        tokio::spawn(async move { apply_with_executable(&task_state, request, &missing).await });
    queued(&state).await;
    let path = state
        .root
        .join(&baseline.id)
        .join("attachments")
        .join(format!("{hash}.step"));
    let mut corrupted = bytes.clone();
    corrupted[0] ^= 1;
    std::fs::write(&path, &corrupted).unwrap();
    // Compare against the externally corrupted snapshot, not pretend the host
    // rolls back an independent file modification made by another actor.
    let before = snapshot(&state, &baseline.id).await;
    drop(hold);
    let error = job.await.unwrap().unwrap_err();
    assert!(
        error.to_string().contains("integrity")
            || error.to_string().contains("ASSET_CHECKSUM_MISMATCH"),
        "{error}"
    );
    assert_eq!(snapshot(&state, &baseline.id).await, before);
    clean(&state).await;
    std::fs::write(path, &bytes).unwrap();
    let stored = forma_core::projects::get(&state, &baseline.id)
        .await
        .unwrap();
    assert_eq!(stored.revisions, baseline.revisions);
}
