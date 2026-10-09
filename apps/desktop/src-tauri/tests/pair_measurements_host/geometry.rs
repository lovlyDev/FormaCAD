use super::fixture::*;
use forma_core::{
    core::AppState,
    model_pair_measurement::{
        build::measure_with_executable,
        schema::{PairMeasurementQuery, PairMeasurementValue},
    },
};
use serde_json::json;
pub async fn clean_pairs(state: &AppState) {
    clean(state).await;
    let path = state.root.join(".transient/pair-measurements");
    if path.exists() {
        assert_eq!(std::fs::read_dir(path).unwrap().count(), 0);
    }
}
#[tokio::test]
async fn same_body_pair_measurement_is_readonly_for_readonly_project_access() {
    let root = tempfile::tempdir().unwrap();
    let (state, baseline) = fixture(root.path()).await;
    let before = snapshot(&state, &baseline.id).await;
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
    let reference = |role: &str| json!({"schemaVersion":1,"kind":"face","ownerFeatureId":"pad","role":role,"occurrencePath":[]});
    let query:PairMeasurementQuery=serde_json::from_value(json!({"kind":"minimumDistance","first":reference("box-face:xmin"),"second":reference("box-face:xmax")})).unwrap();
    let report = measure_with_executable(
        &other,
        &baseline.id,
        baseline.current_revision.as_deref().unwrap(),
        "body",
        &query,
        token(),
        executable(),
    )
    .await
    .unwrap();
    assert_eq!(report.project_id, baseline.id);
    let PairMeasurementValue::MinimumDistance { distance_mm, .. } = report.report.result else {
        panic!("Wrong pair kind")
    };
    assert!((distance_mm - 40.).abs() < 1e-6);
    assert_eq!(snapshot(&state, &baseline.id).await, before);
    assert!(other.grants.lock().await.is_empty());
    clean_pairs(&other).await;
}
