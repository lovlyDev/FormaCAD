use super::tests::fixture;
use super::*;
use crate::{
    cad_ir::{Body, Document, Feature, Operation},
    project_repository::{
        backend::RepositoryRead,
        sqlite_folder::api::{FolderProject, Publication},
    },
};

async fn imported_folder(
    state: &AppState,
    baseline: &crate::models::Project,
    root: &Path,
) -> Arc<FolderProject> {
    let mut project = baseline.clone();
    let revision = project.current_revision.clone().unwrap();
    let source_name = project
        .revisions
        .iter()
        .find(|entry| entry.id == revision)
        .unwrap()
        .source
        .clone()
        .unwrap();
    let source_hash = project
        .files
        .iter()
        .find(|file| file.name == source_name)
        .unwrap()
        .sha256
        .clone()
        .unwrap();
    let document = Document {
        schema_version: 2,
        revision_id: revision.clone(),
        parameters: vec![],
        features: vec![Feature {
            id: "imported".into(),
            name: "Imported fixture".into(),
            suppressed: false,
            operation: Operation::ImportStep {
                asset_id: format!("step_{source_hash}"),
                sha256: source_hash,
            },
        }],
        bodies: vec![Body {
            id: "body".into(),
            name: "Body".into(),
            source_feature_id: "imported".into(),
        }],
    };
    let authored = project
        .revisions
        .iter_mut()
        .find(|entry| entry.id == revision)
        .unwrap();
    authored.program = Some(serde_json::to_string(&document).unwrap());
    // The old mesh describes the box's original authored feature IDs, not this
    // import document. Exact measurements support a saved STEP without preview.
    authored.preview = None;
    let mut pending = Vec::new();
    for file in &project.files {
        pending.push((
            file.sha256.clone().unwrap(),
            crate::artifacts::read(state, &project.id, file).unwrap(),
        ));
    }
    assert!(matches!(
        FolderProject::create(root, project, pending, None)
            .await
            .unwrap(),
        Publication::Committed(_)
    ));
    // The folder owns independent bytes; a managed attachment with the same
    // project/hash/name must never be a fallback for capture or reference staging.
    for file in &baseline.files {
        let hash = file.sha256.as_deref().unwrap();
        let ext = Path::new(&file.name)
            .extension()
            .unwrap()
            .to_str()
            .unwrap()
            .to_ascii_lowercase();
        let path = state
            .root
            .join(&baseline.id)
            .join("attachments")
            .join(format!("{hash}.{ext}"));
        let mut bytes = std::fs::read(&path).unwrap();
        bytes[0] ^= 1;
        std::fs::write(path, bytes).unwrap();
    }
    Arc::new(FolderProject::open(root).await.unwrap())
}

#[tokio::test]
async fn queued_folder_measurement_preserves_metadata_and_stages_actual_import() {
    let app = tempfile::tempdir().unwrap();
    let folder = tempfile::tempdir().unwrap();
    let (state, baseline) = fixture::fixture(app.path()).await;
    let source = imported_folder(&state, &baseline, folder.path()).await;
    let original = source.capture_metadata().await.unwrap();
    let scope = original.task_scope_id().to_owned();
    let revision = original.project().current_revision.clone().unwrap();
    let read = RepositoryRead::from_folder_view(
        Arc::clone(&source),
        source.capture_metadata().await.unwrap(),
    )
    .await
    .unwrap();
    let hold = state
        .cad_tasks
        .acquire(&baseline.id, "hold", fixture::token())
        .await
        .unwrap();
    let job_state = Arc::clone(&state);
    let job_revision = revision.clone();
    let job = tokio::spawn(async move {
        measure_source_with_executable(
            &job_state,
            read,
            &job_revision,
            "body",
            &MeasurementQuery::BodyMetrics {},
            fixture::token(),
            fixture::executable(),
        )
        .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(15), async {
        loop {
            if state.cad_tasks.status().unwrap().iter().any(|task| {
                task.project_id == scope
                    && task.kind == "reference_measurement"
                    && task.state == "queued"
            }) {
                break;
            }
            assert!(
                !job.is_finished(),
                "The real measurement must enter its queue before metadata publication"
            );
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let approval = source.approve_write(&original).await.unwrap();
    let mut metadata = original.project().clone();
    metadata.name = "Folder metadata winner".into();
    metadata.messages.push(crate::models::Message {
        id: uuid::Uuid::new_v4().to_string(),
        role: "assistant".into(),
        text: "Queued metadata".into(),
        created_at: "2026-11-04T00:00:00Z".into(),
    });
    assert!(matches!(
        source
            .commit(&original, approval, metadata.clone(), vec![])
            .await
            .unwrap(),
        Publication::Committed(_)
    ));
    let app_before = fixture::snapshot(&state, &baseline.id).await;
    let folder_before = std::fs::read(folder.path().join("project.sqlite")).unwrap();
    drop(hold);
    let report = job.await.unwrap().unwrap();
    assert_eq!(report.report.imported_asset_seals.len(), 1);
    match report.report.result {
        crate::model_measurement::schema::MeasurementValue::BodyMetrics {
            volume_mm3,
            extents_mm,
            ..
        } => {
            assert!((volume_mm3 - 8000.0).abs() < 1e-6);
            for (a, b) in extents_mm.into_iter().zip([40.0, 20.0, 10.0]) {
                assert!((a - b).abs() < 1e-6);
            }
        }
        _ => panic!("Expected genuine body metrics"),
    }
    assert_eq!(fixture::snapshot(&state, &baseline.id).await, app_before);
    assert_eq!(
        std::fs::read(folder.path().join("project.sqlite")).unwrap(),
        folder_before
    );
    let latest = source.capture_metadata().await.unwrap();
    assert_eq!(latest.project().messages, metadata.messages);
    assert_eq!(latest.project().name, metadata.name);
    fixture::clean(&state).await;
}

#[tokio::test]
async fn folder_close_after_real_measurement_refuses_the_late_report() {
    let app = tempfile::tempdir().unwrap();
    let folder = tempfile::tempdir().unwrap();
    let (state, baseline) = fixture::fixture(app.path()).await;
    let source = imported_folder(&state, &baseline, folder.path()).await;
    let original = source.capture_metadata().await.unwrap();
    let revision = original.project().current_revision.clone().unwrap();
    let read = RepositoryRead::from_folder_view(Arc::clone(&source), original)
        .await
        .unwrap();
    let app_before = fixture::snapshot(&state, &baseline.id).await;
    let folder_before = std::fs::read(folder.path().join("project.sqlite")).unwrap();
    let called = AtomicBool::new(false);
    let after_worker = || {
        called.store(true, Ordering::Relaxed);
        source.close();
    };
    let error = measure_inner_source(
        &state,
        &baseline.id,
        &revision,
        "body",
        &MeasurementQuery::BodyMetrics {},
        fixture::token(),
        fixture::executable(),
        PhaseHook {
            after_worker: Some(&after_worker),
            ..Default::default()
        },
        Some(read),
    )
    .await
    .unwrap_err();
    assert!(called.load(Ordering::Relaxed));
    assert!(error.to_string().contains("MEASUREMENT_STALE"), "{error}");
    assert_eq!(fixture::snapshot(&state, &baseline.id).await, app_before);
    assert_eq!(
        std::fs::read(folder.path().join("project.sqlite")).unwrap(),
        folder_before
    );
    fixture::clean(&state).await;
}
