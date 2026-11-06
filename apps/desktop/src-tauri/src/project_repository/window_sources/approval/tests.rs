use super::*;
#[cfg(feature = "native-worker-client")]
mod worker;
use crate::{
    models::Project,
    project_repository::sqlite_folder::api::{FolderProject, Publication},
};

struct Fixture {
    root: tempfile::TempDir,
    windows: WindowSources,
    id: String,
    key: String,
    commit: String,
}

#[tokio::test]
async fn preparation_uses_original_project_once_and_failed_validation_mints_no_request() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let f = Fixture::new().await;
    let before = f.bytes();
    let calls = AtomicUsize::new(0);
    let prepare = |project: &Project| {
        calls.fetch_add(1, Ordering::Relaxed);
        assert_eq!(project.id, f.id);
        assert_eq!(project.name, "Original");
        Ok(WriteIntent::History(Direction::Undo))
    };
    assert!(f
        .windows
        .request_prepared_write(
            ApprovalSource {
                commit_id: "wrong",
                ..f.source()
            },
            prepare
        )
        .await
        .is_err());
    assert_eq!(calls.load(Ordering::Relaxed), 0);
    let id = f
        .windows
        .request_prepared_write(f.source(), prepare)
        .await
        .unwrap();
    assert_eq!(calls.load(Ordering::Relaxed), 1);
    assert!(f
        .windows
        .request_prepared_write(f.source(), |_| Err(AppError::Invalid(
            "invalid test command".into()
        )))
        .await
        .is_err());
    assert_eq!(f.windows.approvals.0.lock().unwrap().len(), 1);
    f.windows.resolve_write(&id, "window", false).await.unwrap();
    assert_eq!(before, f.bytes());
}
impl Fixture {
    async fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let id = uuid::Uuid::new_v4().to_string();
        let project: Project = serde_json::from_value(serde_json::json!({
            "schemaVersion":1,"id":id,"name":"Original","units":"mm","agent":"codex","pinned":false,
            "createdAt":"2026-11-06T00:00:00Z","updatedAt":"2026-11-06T00:00:00Z","currentRevision":null,
            "messages":[],"exports":[],"files":[],"revisions":[]
        })).unwrap();
        assert!(matches!(
            FolderProject::create(root.path(), project, vec![], None)
                .await
                .unwrap(),
            Publication::Committed(_)
        ));
        let windows = WindowSources::default();
        let opened = windows
            .for_window("window")
            .unwrap()
            .open(root.path())
            .await
            .unwrap();
        Self {
            root,
            windows,
            id,
            key: opened.id.to_string(),
            commit: opened.snapshot.commit_id().into(),
        }
    }
    fn source(&self) -> ApprovalSource<'_> {
        ApprovalSource {
            window: "window",
            source_id: &self.key,
            project_id: &self.id,
            commit_id: &self.commit,
        }
    }
    async fn request(&self) -> String {
        self.windows
            .request_write(
                self.source(),
                WriteIntent::Model(ModelIntent {
                    program: "exact original CAD command".into(),
                    prompt: "original prompt".into(),
                }),
            )
            .await
            .unwrap()
    }
    fn bytes(&self) -> Vec<u8> {
        std::fs::read(self.root.path().join("project.sqlite")).unwrap()
    }
}

#[tokio::test]
async fn request_is_read_only_and_retains_exact_typed_intent_until_one_use() {
    let f = Fixture::new().await;
    let before = f.bytes();
    let id = f.request().await;
    assert!(f.windows.consume_write(&id, f.source()).await.is_err());
    f.windows.resolve_write(&id, "window", true).await.unwrap();
    assert!(f.windows.resolve_write(&id, "window", true).await.is_err());
    let approved = f.windows.consume_write(&id, f.source()).await.unwrap();
    assert_eq!(approved.operation.commit_id(), f.commit);
    let WriteIntent::Model(intent) = approved.intent else {
        panic!("intent changed")
    };
    assert_eq!(intent.program, "exact original CAD command");
    assert_eq!(intent.prompt, "original prompt");
    assert!(f.windows.consume_write(&id, f.source()).await.is_err());
    assert_eq!(before, f.bytes());
}

#[tokio::test]
async fn wrong_window_source_project_or_commit_cannot_consume_or_resolve() {
    let f = Fixture::new().await;
    let id = f.request().await;
    assert!(f
        .windows
        .resolve_write(&id, "foreign-window", true)
        .await
        .is_err());
    f.windows.resolve_write(&id, "window", true).await.unwrap();
    for foreign in [
        ApprovalSource {
            window: "foreign-window",
            ..f.source()
        },
        ApprovalSource {
            source_id: "foreign-source",
            ..f.source()
        },
        ApprovalSource {
            project_id: "foreign-project",
            ..f.source()
        },
        ApprovalSource {
            commit_id: "foreign-commit",
            ..f.source()
        },
    ] {
        assert!(f.windows.consume_write(&id, foreign).await.is_err());
    }
    // Foreign attempts neither use nor transfer the original decision.
    f.windows.consume_write(&id, f.source()).await.unwrap();
}

#[tokio::test]
async fn denial_and_expiry_are_terminal_and_do_not_write() {
    let f = Fixture::new().await;
    let before = f.bytes();
    let denied = f.request().await;
    f.windows
        .resolve_write(&denied, "window", false)
        .await
        .unwrap();
    assert!(f
        .windows
        .resolve_write(&denied, "window", true)
        .await
        .is_err());
    assert!(f.windows.consume_write(&denied, f.source()).await.is_err());
    let expired = f.request().await;
    {
        let mut requests = f.windows.approvals.0.lock().unwrap();
        Arc::get_mut(requests.get_mut(&expired).unwrap())
            .unwrap()
            .expires = Instant::now() - Duration::from_secs(1);
    }
    assert!(f
        .windows
        .resolve_write(&expired, "window", true)
        .await
        .is_err());
    assert!(f.windows.consume_write(&expired, f.source()).await.is_err());
    assert_eq!(before, f.bytes());
}

#[tokio::test]
async fn close_reopen_and_reused_window_label_revoke_pending_and_allowed_requests() {
    let f = Fixture::new().await;
    let before = f.bytes();
    let pending = f.request().await;
    let allowed = f.request().await;
    f.windows
        .resolve_write(&allowed, "window", true)
        .await
        .unwrap();
    f.windows.close_window("window");
    let reopened = f
        .windows
        .for_window("window")
        .unwrap()
        .open(f.root.path())
        .await
        .unwrap();
    assert_ne!(reopened.id.to_string(), f.key);
    assert!(f
        .windows
        .resolve_write(&pending, "window", true)
        .await
        .is_err());
    assert!(f.windows.consume_write(&allowed, f.source()).await.is_err());
    assert_eq!(before, f.bytes());
}

#[tokio::test]
async fn changed_saved_state_cannot_be_legalized_by_a_fresh_request_reference() {
    let f = Fixture::new().await;
    let pending = f.request().await;
    let allowed = f.request().await;
    f.windows
        .resolve_write(&allowed, "window", true)
        .await
        .unwrap();
    let operation = f
        .windows
        .capture_operation("window", &f.key, &f.id, &f.commit)
        .await
        .unwrap();
    let approval = operation.approve_write().await.unwrap();
    let mut candidate = operation.project().clone();
    candidate.name = "New saved state".into();
    let Publication::Committed(saved) =
        operation.commit(approval, candidate, vec![]).await.unwrap()
    else {
        panic!("not committed")
    };
    let after = f.bytes();
    assert!(f
        .windows
        .resolve_write(&pending, "window", true)
        .await
        .is_err());
    assert!(f.windows.consume_write(&allowed, f.source()).await.is_err());
    assert!(f
        .windows
        .consume_write(
            &allowed,
            ApprovalSource {
                commit_id: saved.commit_id(),
                ..f.source()
            }
        )
        .await
        .is_err());
    assert_eq!(after, f.bytes());
}

#[tokio::test]
async fn concurrent_consumers_receive_only_one_original_command() {
    let f = Fixture::new().await;
    let id = f.request().await;
    f.windows.resolve_write(&id, "window", true).await.unwrap();
    let (first, second) = tokio::join!(
        f.windows.consume_write(&id, f.source()),
        f.windows.consume_write(&id, f.source())
    );
    assert_ne!(first.is_ok(), second.is_ok());
}

#[tokio::test]
async fn per_request_wait_does_not_hold_global_registry_or_other_decisions() {
    let f = Fixture::new().await;
    let first = f.request().await;
    let request = f.windows.approvals.get(&first).unwrap();
    let _held = request.decision.lock().await;
    let second = tokio::time::timeout(Duration::from_secs(5), f.request())
        .await
        .unwrap();
    tokio::time::timeout(
        Duration::from_secs(5),
        f.windows.resolve_write(&second, "window", true),
    )
    .await
    .unwrap()
    .unwrap();
    tokio::time::timeout(
        Duration::from_secs(5),
        f.windows.consume_write(&second, f.source()),
    )
    .await
    .unwrap()
    .unwrap();
}

#[tokio::test]
async fn closing_one_source_preserves_another_same_id_request_and_registry_is_bounded() {
    let f = Fixture::new().await;
    let first = f.request().await;
    let second_source = f
        .windows
        .for_window("window")
        .unwrap()
        .open(f.root.path())
        .await
        .unwrap();
    let key = second_source.id.to_string();
    let source = ApprovalSource {
        source_id: &key,
        ..f.source()
    };
    let second = f
        .windows
        .request_write(source, WriteIntent::History(Direction::Undo))
        .await
        .unwrap();
    f.windows.close_source("window", &f.key).unwrap();
    assert!(f
        .windows
        .resolve_write(&first, "window", true)
        .await
        .is_err());
    f.windows
        .resolve_write(&second, "window", true)
        .await
        .unwrap();
    f.windows.consume_write(&second, source).await.unwrap();
    let seed = f
        .windows
        .request_write(source, WriteIntent::History(Direction::Undo))
        .await
        .unwrap();
    {
        let mut map = f.windows.approvals.0.lock().unwrap();
        let entry = Arc::clone(map.get(&seed).unwrap());
        for n in 1..LIMIT {
            map.insert(format!("synthetic-{n}"), Arc::clone(&entry));
        }
    }
    assert!(f
        .windows
        .request_write(source, WriteIntent::History(Direction::Undo))
        .await
        .is_err());
    assert_eq!(f.windows.approvals.0.lock().unwrap().len(), LIMIT);
}
