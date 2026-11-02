#![cfg(feature = "native-occt")]
//! Host snap route: committed-body capture, isolated real worker, read-only project.
#[allow(dead_code)]
#[path = "reference_measurements_host/fixture.rs"]
mod fixture;
use fixture::*;
use forma_core::model_snap::{build::snap_with_executable, schema::SnapQuery};
use serde_json::json;

fn query(value: serde_json::Value) -> SnapQuery {
    serde_json::from_value(value).unwrap()
}
fn reference(kind: &str, role: &str) -> serde_json::Value {
    json!({"schemaVersion":1,"ownerFeatureId":"pad","occurrencePath":[],"kind":kind,"role":role})
}

#[tokio::test]
async fn exact_snaps_on_the_committed_body_leave_the_project_unchanged() {
    let root = tempfile::tempdir().unwrap();
    let (state, project) = fixture(root.path()).await;
    let revision = project.current_revision.clone().unwrap();
    let before = snapshot(&state, &project.id).await;
    for (value, point, normal) in [
        (
            json!({"kind":"face","reference":reference("face","box-face:zmax"),"rayOriginMm":[3,4,100],"rayDirection":[0,0,-1]}),
            [3., 4., 10.],
            Some([0., 0., 1.]),
        ),
        (
            json!({"kind":"edgeMidpoint","reference":reference("edge","box-edge:x:ymin:zmax")}),
            [0., -10., 10.],
            None,
        ),
    ] {
        let confirmed = snap_with_executable(
            &state,
            &project.id,
            &revision,
            "body",
            &query(value),
            executable(),
        )
        .await
        .unwrap();
        assert_eq!(confirmed.project_id, project.id);
        assert_eq!(confirmed.report.revision_id, revision);
        for (actual, expected) in confirmed.report.result.point_mm.iter().zip(point) {
            assert!((actual - expected).abs() < 1e-7, "{actual} != {expected}");
        }
        assert_eq!(confirmed.report.result.normal.is_some(), normal.is_some());
        if let (Some(actual), Some(expected)) = (confirmed.report.result.normal, normal) {
            for (actual, expected) in actual.iter().zip(expected) {
                assert!((actual - expected).abs() < 1e-9);
            }
        }
    }
    assert_eq!(snapshot(&state, &project.id).await, before);
    let transient = state.root.join(".transient/reference-measurements");
    if transient.exists() {
        assert_eq!(std::fs::read_dir(transient).unwrap().count(), 0);
    }
}

#[tokio::test]
async fn stale_revisions_unknown_bodies_and_invalid_rays_are_rejected_before_the_worker() {
    let root = tempfile::tempdir().unwrap();
    let (state, project) = fixture(root.path()).await;
    let revision = project.current_revision.clone().unwrap();
    let face = query(
        json!({"kind":"face","reference":reference("face","box-face:zmax"),"rayOriginMm":[0,0,100],"rayDirection":[0,0,-1]}),
    );
    let stale = uuid::Uuid::new_v4().to_string();
    assert!(
        snap_with_executable(&state, &project.id, &stale, "body", &face, executable())
            .await
            .is_err()
    );
    assert!(
        snap_with_executable(&state, &project.id, &revision, "missing", &face, executable())
            .await
            .is_err()
    );
    let zero = query(
        json!({"kind":"face","reference":reference("face","box-face:zmax"),"rayOriginMm":[0,0,100],"rayDirection":[0,0,0]}),
    );
    let error = snap_with_executable(&state, &project.id, &revision, "body", &zero, executable())
        .await
        .unwrap_err();
    assert!(error.to_string().contains("INVALID_SNAP_QUERY"), "{error}");
}
