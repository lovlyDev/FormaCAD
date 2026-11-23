//! Linux namespace-loss regression using the real commit route.
use super::{fixture, fs, project, Arc, Duration, FolderSession, ProjectFile};
use crate::project_repository::sqlite_folder::api::{FolderProject, Publication};

#[tokio::test]
async fn vanished_folder_and_external_replacement_cannot_receive_a_late_write() {
    let parent = tempfile::tempdir().unwrap();
    let original_path = parent.path().join("selected");
    let detached_path = parent.path().join("detached");
    let replacement_path = parent.path().join("replacement");
    fs::create_dir(&original_path).unwrap();
    let source = Arc::new(fixture(&original_path).await);
    let opened = source.capture().await.unwrap();
    let base = opened.commit_id().to_owned();
    let project_id = opened.project().id.clone();
    let old_data = fs::read(original_path.join("project.sqlite")).unwrap();

    let worker = Arc::clone(&source);
    let (notify, parked) = tokio::sync::oneshot::channel();
    let (resume, waiting) = std::sync::mpsc::channel();
    let task = tokio::spawn(async move {
        let bytes = b"must not be staged after the source disappears".to_vec();
        let hash = crate::artifacts::digest(&bytes);
        let mut next = opened.project().clone();
        next.name = "Attempt interrupted by media loss".into();
        next.files.push(ProjectFile {
            name: "unpublished.step".into(),
            size: bytes.len() as u64,
            kind: "model".into(),
            data: None,
            sha256: Some(hash.clone()),
        });
        worker
            .commit_inner(
                &opened,
                next,
                vec![(hash, bytes)],
                None,
                Some(Box::new(move || {
                    notify.send(()).unwrap();
                    waiting.recv_timeout(Duration::from_secs(10)).unwrap();
                })),
            )
            .await
    });
    tokio::time::timeout(Duration::from_secs(10), parked)
        .await
        .expect("writer must reach the pre-publication barrier")
        .unwrap();
    fs::rename(&original_path, &detached_path).unwrap();
    assert!(FolderProject::open(&original_path).await.is_err());
    fs::create_dir(&original_path).unwrap();
    let mut replacement = project();
    replacement.id = project_id;
    replacement.name = "Different repository at the selected path".into();
    let Publication::Committed(replacement) =
        FolderProject::create(&original_path, replacement, vec![], None)
            .await
            .unwrap()
    else {
        panic!("replacement fixture must have a valid committed head");
    };
    let replacement_bytes = fs::read(original_path.join("project.sqlite")).unwrap();
    assert_ne!(replacement.commit_id(), base);

    resume.send(()).unwrap();
    assert!(task.await.unwrap().is_err());
    assert_eq!(
        fs::read(detached_path.join("project.sqlite")).unwrap(),
        old_data
    );
    assert_eq!(
        fs::read(original_path.join("project.sqlite")).unwrap(),
        replacement_bytes
    );
    for path in [&detached_path, &original_path] {
        assert!(fs::read_dir(path.join("assets")).unwrap().next().is_none());
        assert!(!path.join("project.sqlite-journal").exists());
    }

    let external = FolderSession::open(&detached_path).await.unwrap();
    let external_opening = external.capture().await.unwrap();
    assert_eq!(external_opening.commit_id(), base);
    let mut edited = external_opening.project().clone();
    edited.name = "Saved externally while detached".into();
    let saved = external
        .commit(&external_opening, edited, vec![], None)
        .await
        .unwrap();
    assert!(matches!(saved, super::format::CommitOutcome::Committed(_)));
    let external_head = external.capture().await.unwrap().commit_id().to_owned();
    drop(external);

    fs::rename(&original_path, &replacement_path).unwrap();
    fs::rename(&detached_path, &original_path).unwrap();
    assert!(source.capture().await.is_err());
    let reopened = FolderProject::open(&original_path).await.unwrap();
    let reopened = reopened.capture().await.unwrap();
    assert_eq!(reopened.commit_id(), external_head);
    assert_eq!(reopened.project().name, "Saved externally while detached");
}
