use super::*;
use crate::{
    models::Project,
    project_repository::{
        legacy::read_test_fixture as fixture,
        sqlite_folder::api::{FolderProject, Publication},
    },
};

async fn independent(state: &AppState, root: &Path) -> Arc<FolderProject> {
    let mut project: Project = tests::saved_step(state).await;
    let pending = crate::artifacts::normalize(&mut project).unwrap();
    assert!(matches!(
        FolderProject::create(root, project, pending, None)
            .await
            .unwrap(),
        Publication::Committed(_)
    ));
    Arc::new(FolderProject::open(root).await.unwrap())
}

#[tokio::test]
async fn independent_step_properties_use_original_folder_and_refuse_late_close() {
    let app = tempfile::tempdir().unwrap();
    let folder = tempfile::tempdir().unwrap();
    let state = fixture::state(app.path()).await;
    let source = independent(&state, folder.path()).await;
    let original = source.capture_metadata().await.unwrap();
    let id = original.project().id.clone();
    let revision = original.project().current_revision.clone().unwrap();
    // A conflicting managed row sharing the identity cannot supply our bytes.
    let mut wrong_managed = original.project().clone();
    let wrong = b"intentionally invalid managed STEP";
    wrong_managed.files[0].data = Some(crate::artifacts::data_url(
        &wrong_managed.files[0].name,
        wrong,
    ));
    wrong_managed.files[0].size = wrong.len() as u64;
    wrong_managed.files[0].sha256 = Some(crate::artifacts::digest(wrong));
    fixture::replace(&state, &wrong_managed, false).await;
    let before_app = fixture::logical(&state, &id).await;
    let before_folder = std::fs::read(folder.path().join("project.sqlite")).unwrap();
    let read = RepositoryRead::from_folder_view(Arc::clone(&source), original)
        .await
        .unwrap();
    let properties = run_bound(&state, read, &revision, |p, c| async move {
        crate::native::inspection::inspect_with_executable(&p, c, &tests::worker()).await
    })
    .await
    .unwrap()
    .unwrap();
    assert!((properties.volume_mm3 - 8000.0).abs() < 1e-6);
    for (actual, expected) in properties.bounds_mm.into_iter().zip([40.0, 20.0, 10.0]) {
        assert!((actual - expected).abs() < 1e-6);
    }
    let read = RepositoryRead::from_folder_view(
        Arc::clone(&source),
        source.capture_metadata().await.unwrap(),
    )
    .await
    .unwrap();
    let result = run_bound(&state, read, &revision, |p, c| {
        let source = Arc::clone(&source);
        async move {
            let properties =
                crate::native::inspection::inspect_with_executable(&p, c, &tests::worker()).await?;
            source.close();
            Ok(properties)
        }
    })
    .await;
    assert!(
        matches!(result,Err(AppError::Invalid(ref code)) if code == "PROJECT_SNAPSHOT_CHANGED")
    );
    assert_eq!(before_app, fixture::logical(&state, &id).await);
    assert_eq!(
        before_folder,
        std::fs::read(folder.path().join("project.sqlite")).unwrap()
    );
}
