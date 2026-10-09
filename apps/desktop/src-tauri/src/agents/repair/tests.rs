use super::*;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Mutex,
};

fn geometry_error() -> AppError {
    AppError::Invalid(r#"{"code":"FILLET_FAILED","message":"radius too large"}"#.into())
}
fn candidate() -> PlanResult {
    PlanResult {
        message: "Candidate".into(),
        program: Some("unsaved candidate".into()),
        review_plan: None,
    }
}

#[tokio::test]
async fn allows_two_corrections_at_most_three_builds_from_the_original_input() {
    for failures in 0..=3 {
        let plans = Arc::new(AtomicUsize::new(0));
        let builds = Arc::new(AtomicUsize::new(0));
        let inputs = Arc::new(Mutex::new(Vec::new()));
        let p = plans.clone();
        let b = builds.clone();
        let captured = inputs.clone();
        let result = engine::run(
            "CAPTURED_COMMIT: original",
            Arc::new(AtomicBool::new(false)),
            || async { Ok(()) },
            move |input| {
                p.fetch_add(1, Ordering::Relaxed);
                captured.lock().unwrap().push(input);
                async { Ok(candidate()) }
            },
            move |_, attempt| {
                b.fetch_add(1, Ordering::Relaxed);
                async move {
                    if attempt < failures {
                        Err(geometry_error())
                    } else {
                        Ok(true)
                    }
                }
            },
        )
        .await;
        let expected = (failures + 1).min(3);
        assert_eq!(plans.load(Ordering::Relaxed), expected);
        assert_eq!(builds.load(Ordering::Relaxed), expected);
        assert_eq!(result.is_ok(), failures < 3);
        let inputs = inputs.lock().unwrap();
        for input in inputs.iter() {
            assert!(input.starts_with("CAPTURED_COMMIT: original"));
            assert!(!input.contains("unsaved candidate"));
        }
        if failures >= 2 {
            assert!(inputs[2].contains("Correction 2 of 2"));
        }
        if failures == 3 {
            assert!(result
                .unwrap_err()
                .to_string()
                .contains("CAD_REPAIR_EXHAUSTED"));
        }
    }
}

#[tokio::test]
async fn denied_guard_or_already_cancelled_request_never_invokes_provider_or_builder() {
    for cancelled in [false, true] {
        let plans = Arc::new(AtomicUsize::new(0));
        let p = plans.clone();
        let builds = Arc::new(AtomicUsize::new(0));
        let b = builds.clone();
        let result = engine::run(
            "base",
            Arc::new(AtomicBool::new(cancelled)),
            || async { Err(AppError::Permission("run_agent".into())) },
            move |_| {
                p.fetch_add(1, Ordering::Relaxed);
                async { Ok(candidate()) }
            },
            move |_, _| {
                b.fetch_add(1, Ordering::Relaxed);
                async { Ok(true) }
            },
        )
        .await;
        assert!(result.is_err());
        assert_eq!(plans.load(Ordering::Relaxed), 0);
        assert_eq!(builds.load(Ordering::Relaxed), 0);
    }
}

#[tokio::test]
async fn stale_base_after_provider_or_build_blocks_any_retry_and_acceptance() {
    for fail_check in [2, 3] {
        let checks = Arc::new(AtomicUsize::new(0));
        let c = checks.clone();
        let plans = Arc::new(AtomicUsize::new(0));
        let p = plans.clone();
        let builds = Arc::new(AtomicUsize::new(0));
        let b = builds.clone();
        let result = engine::run(
            "base",
            Arc::new(AtomicBool::new(false)),
            move || {
                let count = c.fetch_add(1, Ordering::Relaxed) + 1;
                async move {
                    if count == fail_check {
                        Err(AppError::Invalid(
                            "Project changed; retry the CAD edit".into(),
                        ))
                    } else {
                        Ok(())
                    }
                }
            },
            move |_| {
                p.fetch_add(1, Ordering::Relaxed);
                async { Ok(candidate()) }
            },
            move |_, _| {
                b.fetch_add(1, Ordering::Relaxed);
                async { Err(geometry_error()) }
            },
        )
        .await;
        assert!(result
            .err()
            .unwrap()
            .to_string()
            .contains("Project changed"));
        assert_eq!(plans.load(Ordering::Relaxed), 1);
        assert_eq!(
            builds.load(Ordering::Relaxed),
            if fail_check == 2 { 0 } else { 1 }
        );
    }
}

#[tokio::test]
async fn cancellation_during_failed_build_wins_over_geometry_retry() {
    let cancel = Arc::new(AtomicBool::new(false));
    let stop = cancel.clone();
    let plans = Arc::new(AtomicUsize::new(0));
    let p = plans.clone();
    let result = engine::run(
        "base",
        cancel,
        || async { Ok(()) },
        move |_| {
            p.fetch_add(1, Ordering::Relaxed);
            async { Ok(candidate()) }
        },
        move |_, _| {
            stop.store(true, Ordering::Relaxed);
            async { Err(geometry_error()) }
        },
    )
    .await;
    assert!(result.err().unwrap().to_string().contains("Task cancelled"));
    assert_eq!(plans.load(Ordering::Relaxed), 1);
}

#[tokio::test]
async fn permission_asset_integrity_and_worker_protocol_failures_are_not_repaired() {
    for code in [
        "PERMISSION_DENIED",
        "REVISION_CONFLICT",
        "CANCELLED",
        "ASSET_CHECKSUM_MISMATCH",
        "ASSET_MISSING",
        "WORKER_PROTOCOL_INVALID",
        "INVALID_EDGE_REFERENCE",
        "EDGE_REFERENCE_UNRESOLVED",
    ] {
        let plans = Arc::new(AtomicUsize::new(0));
        let p = plans.clone();
        let result = engine::run(
            "base",
            Arc::new(AtomicBool::new(false)),
            || async { Ok(()) },
            move |_| {
                p.fetch_add(1, Ordering::Relaxed);
                async { Ok(candidate()) }
            },
            move |_, _| async move {
                Err(AppError::Invalid(
                    serde_json::json!({"code":code,"message":"blocked"}).to_string(),
                ))
            },
        )
        .await;
        assert!(result.err().unwrap().to_string().contains(code));
        assert_eq!(plans.load(Ordering::Relaxed), 1);
    }
    let plans = Arc::new(AtomicUsize::new(0));
    let p = plans.clone();
    let result = engine::run(
        "base",
        Arc::new(AtomicBool::new(false)),
        || async { Ok(()) },
        move |_| {
            p.fetch_add(1, Ordering::Relaxed);
            async { Err(AppError::Permission("run_agent".into())) }
        },
        |_, _| async { Ok(true) },
    )
    .await;
    assert!(matches!(result, Err(AppError::Permission(_))));
    assert_eq!(plans.load(Ordering::Relaxed), 1);
}

#[tokio::test]
async fn text_only_response_and_non_ir_candidate_do_not_retry() {
    let result = engine::run(
        "base",
        Arc::new(AtomicBool::new(false)),
        || async { Ok(()) },
        |_| async {
            Ok(PlanResult {
                message: "Explanation".into(),
                program: None,
                review_plan: None,
            })
        },
        |_, _| async { panic!("text-only response must not build") },
    )
    .await
    .unwrap();
    assert!(result.program.is_none());
    assert!(engine::run(
        "base",
        Arc::new(AtomicBool::new(false)),
        || async { Ok(()) },
        |_| async { Ok(candidate()) },
        |_, _| async { Ok(false) }
    )
    .await
    .is_err());
}

#[test]
fn only_known_geometry_errors_return_bounded_feedback() {
    assert!(geometry_feedback(&geometry_error())
        .unwrap()
        .contains("FILLET_FAILED"));
    assert!(geometry_feedback(&AppError::Invalid("unrelated".into())).is_none());
    assert!(geometry_feedback(&AppError::Permission("run_agent".into())).is_none());
    assert!(
        geometry_feedback(&AppError::Invalid(r#"{"code":"NEW_UNKNOWN_ERROR"}"#.into())).is_none()
    );
    let error = AppError::Invalid(
        serde_json::json!({"code":"FILLET_FAILED","message":"я".repeat(500)}).to_string(),
    );
    let feedback: serde_json::Value =
        serde_json::from_str(&geometry_feedback(&error).unwrap()).unwrap();
    assert_eq!(feedback["detail"].as_str().unwrap().chars().count(), 300);
}

#[tokio::test]
async fn correction_cannot_drop_or_loosen_the_initial_expected_checks() {
    use crate::agents::review_plan::{ExpectedCheck, ReviewPlan};
    let mut original = candidate();
    original.review_plan = Some(ReviewPlan {
        assumptions: vec![],
        dimensions: vec![],
        affected_body_ids: vec![],
        expected_checks: vec![ExpectedCheck::Volume {
            value_mm3: 8000.0,
            tolerance_mm3: 0.01,
        }],
    });
    let expected = original.review_plan.clone();
    let plans = Arc::new(AtomicUsize::new(0));
    let p = plans.clone();
    let builds = Arc::new(AtomicUsize::new(0));
    let b = builds.clone();
    let result=engine::run("same commit",Arc::new(AtomicBool::new(false)),|| async {Ok(())},
        move |_| {
            let attempt=p.fetch_add(1,Ordering::Relaxed);
            let mut result=original.clone();
            if attempt==1 {result.review_plan=None;}
            if attempt==2 {result.review_plan.as_mut().unwrap().expected_checks.clear();}
            async move {Ok(result)}
        },
        move |candidate,attempt| {
            assert_eq!(candidate.review_plan,expected);
            b.fetch_add(1,Ordering::Relaxed);
            async move {if attempt==2 {Ok(true)} else {
                Err(AppError::Invalid(serde_json::json!({"code":"EXPECTED_CHECK_FAILED","message":"check failed"}).to_string()))
            }}
        }
    ).await.unwrap();
    assert_eq!(builds.load(Ordering::Relaxed), 3);
    assert_eq!(result.review_plan.unwrap().expected_checks.len(), 1);
}
