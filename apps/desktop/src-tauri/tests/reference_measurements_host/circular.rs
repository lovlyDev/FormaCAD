use super::fixture::*;
use forma_core::{
    model_measurement::{
        build::measure_with_executable,
        schema::{MeasurementQuery, MeasurementValue},
    },
    modeling::apply::apply_with_executable,
};
use serde_json::json;

#[tokio::test]
async fn circular_diameter_measurement_is_readonly_and_rechecks_saved_head_after_resize() {
    let root = tempfile::tempdir().unwrap();
    let (state, initial) = fixture(root.path()).await;
    let mut current = initial;
    for radius in [10., 17.] {
        let doc = json!({"schemaVersion":2,"revisionId":"candidate","parameters":[],"features":[
            {"id":"profile","name":"Profile","operation":{"type":"circle","radius":{"kind":"literal","mm":radius}}},
            {"id":"pad","name":"Pad","operation":{"type":"extrude","sketchId":"profile","distance":{"kind":"literal","mm":25}}}
        ],"bodies":[{"id":"body","name":"Body","sourceFeatureId":"pad"}]});
        let previous = current.current_revision.clone().unwrap();
        current = apply_with_executable(&state, request(&current, doc), executable())
            .await
            .unwrap()
            .project;
        let before = snapshot(&state, &current.id).await;
        let query: MeasurementQuery=serde_json::from_value(json!({"kind":"edgeDiameter","reference":{"schemaVersion":1,"kind":"edge","ownerFeatureId":"pad","role":"cylinder-edge:top","occurrencePath":[]}})).unwrap();
        let report = measure_with_executable(
            &state,
            &current.id,
            current.current_revision.as_deref().unwrap(),
            "body",
            &query,
            token(),
            executable(),
        )
        .await
        .unwrap();
        let MeasurementValue::EdgeDiameter { diameter_mm } = report.report.result else {
            panic!("Wrong result")
        };
        assert!((diameter_mm - 2. * radius).abs() < 1e-8);
        assert_eq!(snapshot(&state, &current.id).await, before);
        super::geometry::clean_measurements(&state).await;
        let error = measure_with_executable(
            &state,
            &current.id,
            &previous,
            "body",
            &query,
            token(),
            executable(),
        )
        .await
        .unwrap_err();
        assert!(error.to_string().contains("MEASUREMENT_STALE"), "{error}");
        assert_eq!(snapshot(&state, &current.id).await, before);
        super::geometry::clean_measurements(&state).await;
    }
}
