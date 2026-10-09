use super::fixture::*;
use forma_core::model_measurement::{
    build::measure_with_executable,
    schema::{MeasurementQuery, MeasurementValue},
};
use serde_json::json;

#[tokio::test]
async fn exact_measurement_is_readonly_and_accepts_readonly_project_access() {
    let root = tempfile::tempdir().unwrap();
    let (state, baseline) = fixture(root.path()).await;
    let before = snapshot(&state, &baseline.id).await;
    let permissions: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM permissions")
        .fetch_one(&state.pool)
        .await
        .unwrap();
    state
        .project_access
        .acquire(&state.root, &baseline.id)
        .unwrap();
    let other = super::fixture::state(root.path()).await;
    assert_eq!(
        other
            .project_access
            .acquire(&other.root, &baseline.id)
            .unwrap()
            .mode,
        "read_only"
    );
    let report = measure_with_executable(
        &other,
        &baseline.id,
        baseline.current_revision.as_deref().unwrap(),
        "body",
        &MeasurementQuery::BodyMetrics {},
        token(),
        executable(),
    )
    .await
    .unwrap();
    assert_eq!(report.project_id, baseline.id);
    match report.report.result {
        MeasurementValue::BodyMetrics {
            volume_mm3,
            area_mm2,
            extents_mm,
            face_count,
            edge_count,
        } => {
            assert!((volume_mm3 - 8000.).abs() < 1e-6);
            assert!((area_mm2 - 2800.).abs() < 1e-6);
            for (actual, expected) in extents_mm.into_iter().zip([40., 20., 10.]) {
                assert!((actual - expected).abs() < 1e-5);
            }
            assert_eq!((face_count, edge_count), (6, 12));
        }
        _ => panic!("Wrong measurement kind"),
    }
    assert_eq!(snapshot(&state, &baseline.id).await, before);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM permissions")
            .fetch_one(&state.pool)
            .await
            .unwrap(),
        permissions
    );
    assert!(other.grants.lock().await.is_empty());
    clean_measurements(&other).await;
}

pub async fn clean_measurements(state: &forma_core::core::AppState) {
    clean(state).await;
    let directory = state.root.join(".transient/reference-measurements");
    if directory.exists() {
        assert_eq!(std::fs::read_dir(directory).unwrap().count(), 0);
    }
}

#[tokio::test]
async fn actual_kernel_reference_errors_leave_saved_bytes_and_history_unchanged() {
    let root = tempfile::tempdir().unwrap();
    let (state, baseline) = fixture(root.path()).await;
    let before = snapshot(&state, &baseline.id).await;
    for (owner, expected) in [
        ("another_pad", "TOPOLOGY_REFERENCE_UNRESOLVED"),
        ("pad", "MEASUREMENT_UNSUPPORTED"),
    ] {
        let query: MeasurementQuery = serde_json::from_value(json!({"kind":"edgeRadius","reference":{
            "schemaVersion":1,"kind":"edge","ownerFeatureId":owner,"role":"box-edge:x:ymin:zmax","occurrencePath":[]}})).unwrap();
        assert!(query.valid());
        let error = measure_with_executable(
            &state,
            &baseline.id,
            baseline.current_revision.as_deref().unwrap(),
            "body",
            &query,
            token(),
            executable(),
        )
        .await
        .unwrap_err();
        assert!(error.to_string().contains(expected), "{error}");
        assert_eq!(snapshot(&state, &baseline.id).await, before);
        clean_measurements(&state).await;
    }
    let query: MeasurementQuery = serde_json::from_value(json!({"kind":"planarFace","reference":{
        "schemaVersion":1,"kind":"face","ownerFeatureId":"pad","role":"box-face:xmin","occurrencePath":[]}})).unwrap();
    let report = measure_with_executable(
        &state,
        &baseline.id,
        baseline.current_revision.as_deref().unwrap(),
        "body",
        &query,
        token(),
        executable(),
    )
    .await
    .unwrap();
    match report.report.result {
        MeasurementValue::PlanarFace {
            area_mm2,
            origin_mm,
            normal,
        } => {
            assert!((area_mm2 - 200.).abs() < 1e-6);
            for (actual, expected) in origin_mm.into_iter().zip([-20., 0., 5.]) {
                assert!((actual - expected).abs() < 1e-6);
            }
            for (actual, expected) in normal.into_iter().zip([-1., 0., 0.]) {
                assert!((actual - expected).abs() < 1e-6);
            }
        }
        _ => panic!("Wrong planar result"),
    }
    assert_eq!(snapshot(&state, &baseline.id).await, before);
    clean_measurements(&state).await;
}
