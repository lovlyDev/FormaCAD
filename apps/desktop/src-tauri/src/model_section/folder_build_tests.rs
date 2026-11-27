use super::tests::fixture;
use super::*;
use crate::project_repository::sqlite_folder::api::{FolderProject, Publication};

#[tokio::test]
async fn raw_folder_step_section_uses_original_source_and_refuses_late_close() {
    let app = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let (state, baseline) = fixture::fixture(app.path()).await;
    let mut project = baseline.clone();
    let revision = project.current_revision.clone().unwrap();
    let current = project
        .revisions
        .iter_mut()
        .find(|r| r.id == revision)
        .unwrap();
    current.program = None;
    current.preview = None;
    let source_name = current.source.clone().unwrap();
    let mut pending = Vec::new();
    for file in &project.files {
        pending.push((
            file.sha256.clone().unwrap(),
            crate::artifacts::read(&state, &project.id, file).unwrap(),
        ));
    }
    assert!(matches!(
        FolderProject::create(root.path(), project, pending, None)
            .await
            .unwrap(),
        Publication::Committed(_)
    ));
    let source = Arc::new(FolderProject::open(root.path()).await.unwrap());
    let saved = baseline
        .files
        .iter()
        .find(|f| f.name == source_name)
        .unwrap();
    let hash = saved.sha256.as_deref().unwrap();
    let extension = Path::new(&saved.name)
        .extension()
        .unwrap()
        .to_str()
        .unwrap()
        .to_ascii_lowercase();
    let managed = state
        .root
        .join(&baseline.id)
        .join("attachments")
        .join(format!("{hash}.{extension}"));
    let mut wrong = std::fs::read(&managed).unwrap();
    wrong[0] ^= 1;
    std::fs::write(managed, wrong).unwrap();
    let before_app = fixture::snapshot(&state, &baseline.id).await;
    let before_folder = std::fs::read(root.path().join("project.sqlite")).unwrap();
    let plane = SectionPlane {
        origin_mm: [0., 0., 5.],
        normal: [0., 0., 1.],
        deflection_mm: 0.01,
    };
    let original = RepositoryRead::from_folder_view(
        Arc::clone(&source),
        source.capture_metadata().await.unwrap(),
    )
    .await
    .unwrap();
    assert!(original
        .project()
        .revisions
        .iter()
        .find(|r| r.id == revision)
        .unwrap()
        .program
        .is_none());
    let report = section_source_with_executable(
        &state,
        original,
        &revision,
        &plane,
        fixture::token(),
        fixture::executable(),
    )
    .await
    .unwrap();
    assert_eq!(report.source_sha256, hash);
    assert!((report.geometry.total_length_mm - 120.).abs() < 1e-6);
    let original = RepositoryRead::from_folder_view(
        Arc::clone(&source),
        source.capture_metadata().await.unwrap(),
    )
    .await
    .unwrap();
    let called = AtomicBool::new(false);
    let after_worker = || {
        called.store(true, Ordering::Relaxed);
        source.close();
    };
    let error = section_inner_source(
        &state,
        &baseline.id,
        &revision,
        &plane,
        fixture::token(),
        fixture::executable(),
        PhaseHook {
            after_worker: Some(&after_worker),
            ..Default::default()
        },
        Some(original),
    )
    .await
    .unwrap_err();
    assert!(called.load(Ordering::Relaxed));
    assert!(error.to_string().contains("SECTION_STALE"), "{error}");
    assert_eq!(fixture::snapshot(&state, &baseline.id).await, before_app);
    assert_eq!(
        std::fs::read(root.path().join("project.sqlite")).unwrap(),
        before_folder
    );
    fixture::clean(&state).await;
    assert_eq!(
        std::fs::read_dir(state.root.join(".transient/sections"))
            .unwrap()
            .count(),
        0
    );
}
