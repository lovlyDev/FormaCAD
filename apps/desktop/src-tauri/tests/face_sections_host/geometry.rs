use super::fixture::*;
use forma_core::{
    core::AppState,
    model_face_section::{build::section_with_executable, schema::FaceSectionQuery},
};
use serde_json::json;
pub async fn clean_sections(state: &AppState) {
    clean(state).await;
    let path = state.root.join(".transient/face-sections");
    if path.exists() {
        assert_eq!(std::fs::read_dir(path).unwrap().count(), 0);
    }
}
#[tokio::test]
async fn selected_face_section_is_readonly_for_readonly_project_access() {
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
    let query:FaceSectionQuery=serde_json::from_value(json!({"reference":{"schemaVersion":1,"kind":"face","ownerFeatureId":"pad","role":"box-face:zmax","occurrencePath":[]},"offsetMm":-5,"deflectionMm":0.01})).unwrap();
    let report = section_with_executable(
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
    assert!((report.report.face.area_mm2 - 800.).abs() < 1e-8);
    assert!((report.report.geometry.total_length_mm - 120.).abs() < 1e-6);
    for (actual, expected) in report
        .report
        .geometry
        .plane
        .origin_mm
        .into_iter()
        .zip([0., 0., 5.])
    {
        assert!((actual - expected).abs() < 1e-8);
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
    clean_sections(&other).await;
}
