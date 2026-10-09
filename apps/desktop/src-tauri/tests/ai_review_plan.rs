#![cfg(feature = "native-occt")]
use forma_core::{
    agents::review_plan::{check_metrics, ExpectedCheck, ReviewPlan},
    cad_ir::Document,
    native::worker::build_with_executable,
};
use std::{
    path::Path,
    sync::{atomic::AtomicBool, Arc},
};

#[tokio::test]
async fn expected_checks_use_real_worker_geometry_and_reject_a_different_volume() {
    let root = tempfile::tempdir().unwrap();
    let committed = root.path().join("saved-revision.json");
    std::fs::write(&committed, b"unchanged committed model").unwrap();
    let candidate = root.path().join("candidate");
    std::fs::create_dir(&candidate).unwrap();
    let source = include_str!("../../../../docs/fixtures/parameterized-disc.cad.json");
    let document: Document = serde_json::from_str(source).unwrap();
    assert!(build_with_executable(
        source,
        &candidate,
        Arc::new(AtomicBool::new(false)),
        Path::new(env!("CARGO_BIN_EXE_forma-cad-worker"))
    )
    .await
    .unwrap());
    let result: serde_json::Value =
        serde_json::from_slice(&std::fs::read(candidate.join("result.json")).unwrap()).unwrap();
    let volume = result["volumeMm3"].as_f64().unwrap();
    let bounds: [f64; 3] = serde_json::from_value(result["boundsMm"].clone()).unwrap();
    assert!(volume > 0.0);
    let mut plan = ReviewPlan {
        assumptions: vec![],
        dimensions: vec![],
        affected_body_ids: vec![document.bodies[0].id.clone()],
        expected_checks: vec![
            ExpectedCheck::ValidSolid,
            ExpectedCheck::BodyCount {
                count: document.bodies.len(),
            },
            ExpectedCheck::Bounds {
                size_mm: bounds,
                tolerance_mm: 0.001,
            },
            ExpectedCheck::Volume {
                value_mm3: volume,
                tolerance_mm3: 0.001,
            },
        ],
    };
    plan.validate(&document, None).unwrap();
    check_metrics(&plan, &document, &result).unwrap();
    plan.expected_checks[3] = ExpectedCheck::Volume {
        value_mm3: volume * 1.1,
        tolerance_mm3: 0.001,
    };
    let error = check_metrics(&plan, &document, &result)
        .unwrap_err()
        .to_string();
    assert!(error.contains("EXPECTED_CHECK_FAILED"));
    assert!(error.contains("actual"));
    assert_eq!(
        std::fs::read(committed).unwrap(),
        b"unchanged committed model"
    );
    assert!(candidate.join("model.step").is_file());
    assert!(candidate.join("preview.glb").is_file());
}
