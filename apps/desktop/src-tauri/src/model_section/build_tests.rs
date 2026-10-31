use super::*;
#[allow(dead_code)]
mod fixture {
    use crate as forma_core;
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/reference_measurements_host/fixture.rs"
    ));
}

fn plane() -> SectionPlane {
    SectionPlane {
        origin_mm: [0., 0., 5.],
        normal: [0., 0., 1.],
        deflection_mm: 0.01,
    }
}

#[tokio::test]
async fn close_and_reopen_after_verified_worker_rejects_late_manual_section() {
    let root = tempfile::tempdir().unwrap();
    let (state, baseline) = fixture::fixture(root.path()).await;
    let before = fixture::snapshot(&state, &baseline.id).await;
    let called = AtomicBool::new(false);
    let reopen_after_worker = || {
        called.store(true, Ordering::Relaxed);
        state.project_access.release(&baseline.id).unwrap();
        state
            .project_access
            .acquire(&state.root, &baseline.id)
            .unwrap();
    };
    let error = section_inner(
        &state,
        &baseline.id,
        baseline.current_revision.as_deref().unwrap(),
        &plane(),
        fixture::token(),
        fixture::executable(),
        PhaseHook {
            after_worker: Some(&reopen_after_worker),
            ..Default::default()
        },
    )
    .await
    .unwrap_err();
    assert!(
        called.load(Ordering::Relaxed),
        "The actual section worker must finish before reopen"
    );
    assert!(error.to_string().contains("SECTION_STALE"), "{error}");
    assert_eq!(fixture::snapshot(&state, &baseline.id).await, before);
    fixture::clean(&state).await;
    assert_eq!(
        std::fs::read_dir(state.root.join(".transient/sections"))
            .unwrap()
            .count(),
        0
    );
    let fresh = section_with_executable(
        &state,
        &baseline.id,
        baseline.current_revision.as_deref().unwrap(),
        &plane(),
        fixture::token(),
        fixture::executable(),
    )
    .await
    .unwrap();
    assert!((fresh.geometry.total_length_mm - 120.).abs() < 1e-6);
    let saved = baseline
        .files
        .iter()
        .find(|file| baseline.revisions.last().unwrap().source.as_ref() == Some(&file.name))
        .unwrap();
    assert_eq!(Some(&fresh.source_sha256), saved.sha256.as_ref());
    assert_eq!(fixture::snapshot(&state, &baseline.id).await, before);
    fixture::clean(&state).await;
    state.project_access.release(&baseline.id).unwrap();
}
