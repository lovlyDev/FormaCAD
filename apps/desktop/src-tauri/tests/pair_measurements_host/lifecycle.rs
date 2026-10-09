use super::{fixture::*, geometry::clean_pairs};
use forma_core::{
    model_pair_measurement::{build::measure_with_executable, schema::PairMeasurementQuery},
    models::ProjectFile,
};
use serde_json::json;

#[tokio::test]
async fn queued_metadata_only_save_preserves_messages_and_does_not_invalidate_inputs() {
    let root = tempfile::tempdir().unwrap();
    let (state, baseline) = fixture(root.path()).await;
    let hold = state
        .cad_tasks
        .acquire(&baseline.id, "fixture", token())
        .await
        .unwrap();
    let job_state = state.clone();
    let project = baseline.clone();
    let job = tokio::spawn(async move {
        measure_with_executable(
            &job_state,
            &project.id,
            project.current_revision.as_deref().unwrap(),
            "body",
            &pair_query(),
            token(),
            executable(),
        )
        .await
    });
    queued(&state).await;
    let mut winner = baseline.clone();
    winner.messages.push(forma_core::models::Message {
        id: uuid::Uuid::new_v4().to_string(),
        role: "assistant".into(),
        text: "Independent metadata winner".into(),
        created_at: "2026-10-06T00:00:00Z".into(),
    });
    let writes = state.writes.lock().await;
    let winner = forma_core::projects::persist(&state, winner, vec![], None)
        .await
        .unwrap();
    drop(writes);
    let before = snapshot(&state, &winner.id).await;
    drop(hold);
    let report = job.await.unwrap().unwrap();
    assert_eq!(
        report.report.revision_id,
        baseline.current_revision.unwrap()
    );
    assert_eq!(snapshot(&state, &winner.id).await, before);
    assert_eq!(
        forma_core::projects::get(&state, &winner.id)
            .await
            .unwrap()
            .messages,
        winner.messages
    );
    clean_pairs(&state).await;
}

#[tokio::test]
async fn pending_recovery_is_not_published_or_cleared_by_measurement() {
    let root = tempfile::tempdir().unwrap();
    let (state, baseline) = fixture(root.path()).await;
    state
        .project_access
        .acquire(&state.root, &baseline.id)
        .unwrap();
    let manifest = std::fs::read(state.root.join(&baseline.id).join("manifest.json")).unwrap();
    let project_bytes = serde_json::to_vec(&baseline).unwrap();
    // A valid already-published generation with its journal cleanup outstanding.
    sqlx::query("INSERT INTO project_storage_commits(project_id,base_manifest_sha256,target_manifest,project_sha256) VALUES(?,?,?,?)")
        .bind(&baseline.id).bind(forma_core::artifacts::digest(&manifest))
        .bind(String::from_utf8(manifest).unwrap()).bind(forma_core::artifacts::digest(&project_bytes))
        .execute(&state.pool).await.unwrap();
    let before = snapshot(&state, &baseline.id).await;
    let missing = root.path().join("must-not-spawn.exe");
    let error = measure_with_executable(
        &state,
        &baseline.id,
        baseline.current_revision.as_deref().unwrap(),
        "body",
        &pair_query(),
        token(),
        &missing,
    )
    .await
    .unwrap_err();
    assert!(
        error.to_string().contains("MEASUREMENT_RECOVERY_REQUIRED"),
        "{error}"
    );
    assert_eq!(snapshot(&state, &baseline.id).await, before);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM project_storage_commits WHERE project_id=?"
        )
        .bind(&baseline.id)
        .fetch_one(&state.pool)
        .await
        .unwrap(),
        1
    );
    clean_pairs(&state).await;
}

#[tokio::test]
async fn tampered_raw_program_with_unchanged_head_is_rejected_without_repair() {
    let root = tempfile::tempdir().unwrap();
    let (state, baseline) = fixture(root.path()).await;
    let hold = state
        .cad_tasks
        .acquire(&baseline.id, "fixture", token())
        .await
        .unwrap();
    let job_state = state.clone();
    let project = baseline.clone();
    let missing = root.path().join("must-not-spawn.exe");
    let job = tokio::spawn(async move {
        measure_with_executable(
            &job_state,
            &project.id,
            project.current_revision.as_deref().unwrap(),
            "body",
            &pair_query(),
            token(),
            &missing,
        )
        .await
    });
    queued(&state).await;
    let mut tampered = baseline.clone();
    let revision = tampered.revisions.last_mut().unwrap();
    let mut program: serde_json::Value =
        serde_json::from_str(revision.program.as_ref().unwrap()).unwrap();
    program["features"][0]["operation"]["width"]["mm"] = 52.into();
    revision.program = Some(program.to_string());
    sqlx::query("UPDATE projects SET payload=? WHERE id=?")
        .bind(serde_json::to_string(&tampered).unwrap())
        .bind(&baseline.id)
        .execute(&state.pool)
        .await
        .unwrap();
    // This is an external malicious-index modification, not a legal modeling race.
    let before = snapshot(&state, &baseline.id).await;
    drop(hold);
    let error = job.await.unwrap().unwrap_err();
    assert!(error.to_string().contains("MEASUREMENT_STALE"), "{error}");
    assert_eq!(snapshot(&state, &baseline.id).await, before);
    clean_pairs(&state).await;
}

#[tokio::test]
async fn queued_measurement_cancellation_never_starts_a_worker() {
    let root = tempfile::tempdir().unwrap();
    let (state, baseline) = fixture(root.path()).await;
    let before = snapshot(&state, &baseline.id).await;
    let hold = state
        .cad_tasks
        .acquire(&baseline.id, "fixture", token())
        .await
        .unwrap();
    let job_state = state.clone();
    let project = baseline.clone();
    let missing = root.path().join("must-not-spawn.exe");
    let job = tokio::spawn(async move {
        measure_with_executable(
            &job_state,
            &project.id,
            project.current_revision.as_deref().unwrap(),
            "body",
            &pair_query(),
            token(),
            &missing,
        )
        .await
    });
    queued(&state).await;
    state.cad_tasks.cancel_project(&baseline.id).unwrap();
    let error = job.await.unwrap().unwrap_err();
    assert!(error.to_string().contains("CANCELLED"), "{error}");
    drop(hold);
    assert_eq!(snapshot(&state, &baseline.id).await, before);
    clean_pairs(&state).await;
}

#[tokio::test]
async fn queued_measurement_rejects_a_new_committed_head_before_worker_spawn() {
    let root = tempfile::tempdir().unwrap();
    let (state, baseline) = fixture(root.path()).await;
    let hold = state
        .cad_tasks
        .acquire(&baseline.id, "fixture", token())
        .await
        .unwrap();
    let job_state = state.clone();
    let project = baseline.clone();
    let missing = root.path().join("must-not-spawn.exe");
    let job = tokio::spawn(async move {
        measure_with_executable(
            &job_state,
            &project.id,
            project.current_revision.as_deref().unwrap(),
            "body",
            &pair_query(),
            token(),
            &missing,
        )
        .await
    });
    queued(&state).await;
    let mut winner = baseline.clone();
    let mut revision = winner.revisions.last().unwrap().clone();
    let id = uuid::Uuid::new_v4().to_string();
    revision.id = id.clone();
    revision.parent = winner.current_revision.clone();
    let mut source = document(40.);
    source["revisionId"] = id.clone().into();
    revision.program = Some(source.to_string());
    winner.current_revision = Some(id);
    winner.revisions.push(revision);
    let winner = forma_core::projects::persist(&state, winner, vec![], None)
        .await
        .unwrap();
    let before = snapshot(&state, &winner.id).await;
    drop(hold);
    let error = job.await.unwrap().unwrap_err();
    assert!(error.to_string().contains("MEASUREMENT_STALE"), "{error}");
    assert_eq!(snapshot(&state, &winner.id).await, before);
    clean_pairs(&state).await;
}

#[tokio::test]
async fn queued_imported_corruption_cannot_produce_verified_report() {
    let root = tempfile::tempdir().unwrap();
    let (state, mut baseline) = fixture(root.path()).await;
    let source = baseline.revisions.last().unwrap().source.as_ref().unwrap();
    let file = baseline
        .files
        .iter()
        .find(|file| &file.name == source)
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
    let mut candidate = document(40.);
    candidate["features"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"input","name":"Input","operation":{
        "type":"importStep","assetId":format!("step_{hash}"),"sha256":hash}}));
    let accepted = forma_core::modeling::apply::apply_with_executable(
        &state,
        request(&baseline, candidate),
        executable(),
    )
    .await
    .unwrap();
    let project = accepted.project;
    let hold = state
        .cad_tasks
        .acquire(&project.id, "fixture", token())
        .await
        .unwrap();
    let job_state = state.clone();
    let captured = project.clone();
    let missing = root.path().join("must-not-spawn.exe");
    let job = tokio::spawn(async move {
        measure_with_executable(
            &job_state,
            &captured.id,
            captured.current_revision.as_deref().unwrap(),
            "body",
            &pair_query(),
            token(),
            &missing,
        )
        .await
    });
    queued(&state).await;
    let path = state
        .root
        .join(&project.id)
        .join("attachments")
        .join(format!("{hash}.step"));
    let mut corrupted = bytes.clone();
    corrupted[0] ^= 1;
    std::fs::write(&path, corrupted).unwrap();
    let before = snapshot(&state, &project.id).await;
    drop(hold);
    let error = job.await.unwrap().unwrap_err();
    assert!(
        error.to_string().contains("integrity")
            || error.to_string().contains("CHECKSUM")
            || error.to_string().contains("checksum"),
        "{error}"
    );
    assert_eq!(snapshot(&state, &project.id).await, before);
    clean_pairs(&state).await;
    std::fs::write(path, bytes).unwrap(); // Only restore the external modification in this fixture.
}

async fn queued(state: &forma_core::core::AppState) {
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while !state
            .cad_tasks
            .status()
            .unwrap()
            .iter()
            .any(|task| task.kind == "pair_measurement" && task.state == "queued")
        {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

fn pair_query() -> PairMeasurementQuery {
    serde_json::from_value(json!({"kind":"minimumDistance","first":{"schemaVersion":1,"kind":"face","ownerFeatureId":"pad","role":"box-face:xmin","occurrencePath":[]},"second":{"schemaVersion":1,"kind":"face","ownerFeatureId":"pad","role":"box-face:xmax","occurrencePath":[]}})).unwrap()
}
