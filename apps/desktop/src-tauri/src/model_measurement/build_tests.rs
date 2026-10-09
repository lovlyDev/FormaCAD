use super::*;
#[allow(dead_code)]
mod fixture {
    use crate as forma_core;
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/reference_measurements_host/fixture.rs"
    ));
}

#[tokio::test]
async fn cancellation_after_verified_worker_keeps_storage_unchanged() {
    let root = tempfile::tempdir().unwrap();
    let (state, baseline) = fixture::fixture(root.path()).await;
    let before = fixture::snapshot(&state, &baseline.id).await;
    let cancel = fixture::token();
    let called = AtomicBool::new(false);
    let cancel_after_worker = || {
        called.store(true, Ordering::Relaxed);
        cancel.store(true, Ordering::Relaxed);
    };
    let error = measure_inner(
        &state,
        &baseline.id,
        baseline.current_revision.as_deref().unwrap(),
        "body",
        &MeasurementQuery::BodyMetrics {},
        cancel.clone(),
        fixture::executable(),
        PhaseHook {
            after_worker: Some(&cancel_after_worker),
            ..Default::default()
        },
    )
    .await
    .unwrap_err();
    assert!(
        called.load(Ordering::Relaxed),
        "The real worker must finish before cancellation"
    );
    assert!(error.to_string().contains("CAD_TASK_CANCELLED"), "{error}");
    assert_eq!(fixture::snapshot(&state, &baseline.id).await, before);
    fixture::clean(&state).await;
    let temporary = state.root.join(".transient/reference-measurements");
    assert_eq!(std::fs::read_dir(temporary).unwrap().count(), 0);
}

#[tokio::test]
async fn input_corruption_after_real_worker_is_rejected_by_final_host_check() {
    let root = tempfile::tempdir().unwrap();
    let (state, baseline) = fixture::fixture(root.path()).await;
    let source = baseline.revisions.last().unwrap().source.as_ref().unwrap();
    let file = baseline
        .files
        .iter()
        .find(|file| &file.name == source)
        .unwrap();
    let original = crate::artifacts::read(&state, &baseline.id, file).unwrap();
    let path = state
        .root
        .join(&baseline.id)
        .join("attachments")
        .join(format!("{}.step", file.sha256.as_ref().unwrap()));
    let before: String = sqlx::query_scalar("SELECT payload FROM projects WHERE id=?")
        .bind(&baseline.id)
        .fetch_one(&state.pool)
        .await
        .unwrap();
    let history_before: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM project_history_events WHERE project_id=?")
            .bind(&baseline.id)
            .fetch_one(&state.pool)
            .await
            .unwrap();
    let called = AtomicBool::new(false);
    let fault = || {
        // Called only AFTER the real isolated client verified its successful report.
        called.store(true, Ordering::Relaxed);
        let mut bytes = original.clone();
        bytes[0] ^= 1;
        std::fs::write(&path, bytes).unwrap();
    };
    let error = measure_inner(
        &state,
        &baseline.id,
        baseline.current_revision.as_deref().unwrap(),
        "body",
        &MeasurementQuery::BodyMetrics {},
        fixture::token(),
        fixture::executable(),
        PhaseHook {
            after_worker: Some(&fault),
            ..Default::default()
        },
    )
    .await
    .unwrap_err();
    assert!(
        called.load(Ordering::Relaxed),
        "Fixture must complete the real worker first"
    );
    assert!(
        error.to_string().contains("integrity")
            || error.to_string().contains("CHECKSUM")
            || error.to_string().contains("checksum"),
        "{error}"
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT payload FROM projects WHERE id=?")
            .bind(&baseline.id)
            .fetch_one(&state.pool)
            .await
            .unwrap(),
        before
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM project_history_events WHERE project_id=?"
        )
        .bind(&baseline.id)
        .fetch_one(&state.pool)
        .await
        .unwrap(),
        history_before
    );
    fixture::clean(&state).await;
    assert_eq!(
        std::fs::read_dir(state.root.join(".transient/reference-measurements"))
            .unwrap()
            .count(),
        0
    );
    std::fs::write(path, original).unwrap();
    let valid = measure_with_executable(
        &state,
        &baseline.id,
        baseline.current_revision.as_deref().unwrap(),
        "body",
        &MeasurementQuery::BodyMetrics {},
        fixture::token(),
        fixture::executable(),
    )
    .await
    .unwrap();
    assert_eq!(valid.report.revision_id, baseline.current_revision.unwrap());
}
