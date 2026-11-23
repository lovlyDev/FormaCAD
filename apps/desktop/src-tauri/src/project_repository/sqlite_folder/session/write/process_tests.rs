//! Real process contention and writer termination on a local filesystem.
use super::{fs, project, AppError, Duration, FolderSession, ProjectFile};
use crate::project_repository::sqlite_folder::{
    api::{FolderProject, Publication},
    crash_probe::{Point, POINT, READY},
};
use std::{
    path::{Path, PathBuf},
    process::{Child, Command},
};

const ROOT: &str = "FORMA_FCP1_WRITER_LEASE_TEST_ROOT";
const ORIGINAL_BYTES: &[u8] = b"immutable local process fixture";
const PENDING_BYTES: &[u8] = b"asset supplied by the actual writer transaction";
const CANDIDATE_NAME: &str = "Project committed by the child writer";

#[path = "crash_tests.rs"]
mod crash_tests;

struct ParkedWriter(Child);

impl ParkedWriter {
    async fn start(root: &Path, ready: &Path, point: Point) -> Self {
        let (_, module) = module_path!().split_once("::").unwrap();
        let test = format!("{module}::write_while_holding_the_lease");
        let mut writer = Self(
            Command::new(std::env::current_exe().unwrap())
                .args(["--ignored", "--exact", &test, "--nocapture"])
                .env(ROOT, root)
                .env(POINT, point.name())
                .env(READY, ready)
                .spawn()
                .unwrap(),
        );
        tokio::time::timeout(Duration::from_secs(15), async {
            loop {
                assert!(
                    writer.0.try_wait().unwrap().is_none(),
                    "writer exited before {point:?}"
                );
                if fs::read_to_string(ready).is_ok_and(|value| value == point.name()) {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap_or_else(|_| panic!("writer did not reach {point:?}"));
        writer
    }

    fn kill(&mut self) {
        self.0.kill().unwrap();
        assert!(!self.0.wait().unwrap().success());
    }
}

impl Drop for ParkedWriter {
    fn drop(&mut self) {
        if matches!(self.0.try_wait(), Ok(None)) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

#[tokio::test]
async fn reader_can_copy_but_cannot_write_until_the_other_process_is_killed() {
    let root = tempfile::tempdir().unwrap();
    let control = tempfile::tempdir().unwrap();
    let destination = tempfile::tempdir().unwrap();

    let mut baseline = project();
    let hash = crate::artifacts::digest(ORIGINAL_BYTES);
    baseline.files.push(ProjectFile {
        name: "fixture.step".into(),
        size: ORIGINAL_BYTES.len() as u64,
        kind: "model".into(),
        data: None,
        sha256: Some(hash.clone()),
    });
    let Publication::Committed(baseline) = FolderProject::create(
        root.path(),
        baseline,
        vec![(hash, ORIGINAL_BYTES.to_vec())],
        None,
    )
    .await
    .unwrap() else {
        panic!("expected a committed baseline");
    };
    let mut writer = ParkedWriter::start(
        root.path(),
        &control.path().join("ready"),
        Point::BeforeStaging,
    )
    .await;
    assert_ne!(writer.0.id(), std::process::id());

    let database = root.path().join("project.sqlite");
    let bytes = fs::read(&database).unwrap();
    let reader = FolderProject::open(root.path()).await.unwrap();
    let original = reader.capture().await.unwrap();
    assert_eq!(original.commit_id(), baseline.commit_id());
    let history = reader.read_history(&original).await.unwrap();
    assert_eq!(
        reader
            .read_asset(&original, &original.project().files[0])
            .await
            .unwrap(),
        ORIGINAL_BYTES
    );
    let approval = reader.approve_write(&original).await.unwrap();
    let mut candidate = original.project().clone();
    candidate.name = "Reader writes only after the writer exits".into();
    assert!(matches!(
        reader.commit(&original, approval, candidate.clone(), vec![]).await,
        Err(AppError::Invalid(reason)) if reason == "PROJECT_READ_ONLY"
    ));
    let Publication::Committed(copied) = reader
        .save_copy(&original, destination.path())
        .await
        .unwrap()
    else {
        panic!("reader must be able to publish an independent copy");
    };
    assert_ne!(copied.project().id, original.project().id);
    let copy = FolderProject::open(destination.path()).await.unwrap();
    let copy_view = copy.capture().await.unwrap();
    assert_eq!(
        copy.read_asset(&copy_view, &copy_view.project().files[0])
            .await
            .unwrap(),
        ORIGINAL_BYTES
    );
    assert_eq!(fs::read(&database).unwrap(), bytes);
    assert!(
        writer.0.try_wait().unwrap().is_none(),
        "writer must still hold the lease"
    );

    writer.kill();
    assert!(root.path().join(".forma/writer.lock").is_file());
    assert!(!root.path().join("project.sqlite-journal").exists());
    assert_eq!(fs::read(database).unwrap(), bytes);
    reader.recheck(&original).await.unwrap();
    assert_eq!(reader.read_history(&original).await.unwrap(), history);
    assert_eq!(
        reader
            .read_asset(&original, &original.project().files[0])
            .await
            .unwrap(),
        ORIGINAL_BYTES
    );
    let approval = reader.approve_write(&original).await.unwrap();
    let Publication::Committed(saved) = reader
        .commit(&original, approval, candidate, vec![])
        .await
        .unwrap()
    else {
        panic!("the OS must release the writer lease on termination");
    };
    let reopened = FolderProject::open(root.path()).await.unwrap();
    let reopened = reopened.capture().await.unwrap();
    assert_eq!(reopened.commit_id(), saved.commit_id());
    assert_ne!(reopened.commit_id(), baseline.commit_id());
    assert_eq!(
        reopened.project().name,
        "Reader writes only after the writer exits"
    );
}

#[tokio::test]
#[ignore = "parent process invokes this exact writer fixture"]
async fn write_while_holding_the_lease() {
    let root = PathBuf::from(std::env::var_os(ROOT).expect("parent supplies fixture root"));
    let session = FolderSession::open(&root).await.unwrap();
    let original = session.capture().await.unwrap();
    let mut candidate = original.project().clone();
    candidate.name = CANDIDATE_NAME.into();
    let hash = crate::artifacts::digest(PENDING_BYTES);
    candidate.files.push(ProjectFile {
        name: "child.step".into(),
        kind: "model".into(),
        size: PENDING_BYTES.len() as u64,
        data: None,
        sha256: Some(hash.clone()),
    });
    let outcome = session
        .commit(
            &original,
            candidate,
            vec![(hash, PENDING_BYTES.to_vec())],
            None,
        )
        .await;
    match outcome {
        Ok(_) => panic!("the writer returned without being terminated"),
        Err(error) => panic!("the writer failed before termination: {error}"),
    }
}
