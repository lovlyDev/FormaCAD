use super::*;
use crate::{
    models::Project,
    project_repository::sqlite_folder::api::{FolderProject, Publication},
};
use std::{sync::Arc, time::Duration};
#[cfg(feature = "native-worker-client")]
mod worker;

struct Fixture {
    pool: sqlx::SqlitePool,
    windows: Arc<WindowSources>,
    id: String,
    key: String,
    commit: String,
    root: tempfile::TempDir,
}
impl Fixture {
    async fn new() -> Self {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        let root = tempfile::tempdir().unwrap();
        let id = uuid::Uuid::new_v4().to_string();
        let project:Project=serde_json::from_value(serde_json::json!({
            "schemaVersion":1,"id":id,"name":"Original permission source","units":"mm","agent":"codex","pinned":false,
            "createdAt":"2026-11-06T00:00:00Z","updatedAt":"2026-11-06T00:00:00Z","currentRevision":null,
            "messages":[],"exports":[],"files":[],"revisions":[]
        })).unwrap();
        assert!(matches!(
            FolderProject::create(root.path(), project, vec![], None)
                .await
                .unwrap(),
            Publication::Committed(_)
        ));
        let windows = Arc::new(WindowSources::default());
        let opened = windows.open_for_test("window", root.path()).await.unwrap();
        Self {
            pool,
            windows,
            id,
            key: opened.id.to_string(),
            commit: opened.snapshot.commit_id().into(),
            root,
        }
    }
    fn source(&self) -> Source {
        Source {
            project_id: self.id.clone(),
            source_id: self.key.clone(),
            commit_id: self.commit.clone(),
        }
    }
    fn approval_source(&self) -> ApprovalSource<'_> {
        ApprovalSource {
            window: "window",
            project_id: &self.id,
            source_id: &self.key,
            commit_id: &self.commit,
        }
    }
    fn input(&self) -> Request {
        let program = r#"{"version":1,"features":[{"id":"profile","operation":{"type":"rectangle","width":40,"depth":20}},{"id":"pad","operation":{"type":"extrude","sketch":"profile","distance":10}}],"output":"pad"}"#;
        Request {
            project_id: self.id.clone(),
            source_id: self.key.clone(),
            commit_id: self.commit.clone(),
            detail: "Original command".into(),
            intent: Intent::Model {
                program: program.into(),
                prompt: "Original command".into(),
                expected_revision: None,
                review_plan: None,
            },
        }
    }
    async fn request(&self) -> Receipt {
        request(&self.pool, &self.windows, "window", self.input())
            .await
            .unwrap()
    }
    async fn decision(&self, id: &str) -> String {
        sqlx::query_scalar("SELECT decision FROM permissions WHERE id=?")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }
    fn bytes(&self) -> Vec<u8> {
        std::fs::read(self.root.path().join("project.sqlite")).unwrap()
    }
}

#[tokio::test]
async fn explicit_confirmation_is_original_source_bound_without_managed_project_rows() {
    let f = Fixture::new().await;
    let before = f.bytes();
    let receipt = f.request().await;
    assert!(receipt.requires_confirmation);
    assert_eq!(f.decision(&receipt.id).await, "pending");
    assert!(f
        .windows
        .consume_write(&receipt.id, f.approval_source())
        .await
        .is_err());
    resolve(&f.pool, &f.windows, "window", &receipt.id, true, f.source())
        .await
        .unwrap();
    assert_eq!(f.decision(&receipt.id).await, "allowed");
    f.windows
        .consume_write(&receipt.id, f.approval_source())
        .await
        .unwrap();
    assert!(f
        .windows
        .consume_write(&receipt.id, f.approval_source())
        .await
        .is_err());
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM projects")
        .fetch_one(&f.pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    assert_eq!(before, f.bytes());
}

#[tokio::test]
async fn automatic_decision_comes_only_from_the_server_confirmation_policy() {
    let f = Fixture::new().await;
    sqlx::query("INSERT INTO settings(key,value) VALUES('confirmations',?)")
        .bind(r#"{"mode":"none","overrides":{}}"#)
        .execute(&f.pool)
        .await
        .unwrap();
    let receipt = f.request().await;
    assert!(!receipt.requires_confirmation);
    assert_eq!(f.decision(&receipt.id).await, "automatic");
    f.windows
        .consume_write(&receipt.id, f.approval_source())
        .await
        .unwrap();
    sqlx::query("UPDATE settings SET value=? WHERE key='confirmations'")
        .bind(r#"{"mode":"none","overrides":{"modify_project":true}}"#)
        .execute(&f.pool)
        .await
        .unwrap();
    let receipt = f.request().await;
    assert!(receipt.requires_confirmation);
    assert!(f
        .windows
        .consume_write(&receipt.id, f.approval_source())
        .await
        .is_err());
}

#[tokio::test]
async fn invalid_program_revision_and_commit_create_no_audit_or_permission() {
    let f = Fixture::new().await;
    let mut wrong = f.input();
    wrong.commit_id = "wrong".into();
    assert!(request(&f.pool, &f.windows, "window", wrong).await.is_err());
    let mut wrong = f.input();
    wrong.intent = Intent::Model {
        program: String::new(),
        prompt: String::new(),
        expected_revision: None,
        review_plan: None,
    };
    assert!(request(&f.pool, &f.windows, "window", wrong).await.is_err());
    let mut wrong = f.input();
    wrong.intent = Intent::History {
        direction: super::super::Direction::Undo,
        expected_revision: Some(uuid::Uuid::new_v4().to_string()),
    };
    assert!(request(&f.pool, &f.windows, "window", wrong).await.is_err());
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM permissions")
        .fetch_one(&f.pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn foreign_window_source_project_and_commit_cannot_even_change_audit_decision() {
    let f = Fixture::new().await;
    let receipt = f.request().await;
    for kind in 0..4 {
        let mut source = f.source();
        let window = if kind == 0 { "foreign" } else { "window" };
        match kind {
            1 => source.source_id = "foreign".into(),
            2 => source.project_id = uuid::Uuid::new_v4().to_string(),
            3 => source.commit_id = "foreign".into(),
            _ => {}
        }
        assert!(
            resolve(&f.pool, &f.windows, window, &receipt.id, true, source)
                .await
                .is_err()
        );
        assert_eq!(f.decision(&receipt.id).await, "pending");
    }
    resolve(&f.pool, &f.windows, "window", &receipt.id, true, f.source())
        .await
        .unwrap();
    f.windows
        .consume_write(&receipt.id, f.approval_source())
        .await
        .unwrap();
}

#[tokio::test]
async fn source_close_reopen_and_audit_rows_cannot_resurrect_a_decision() {
    let f = Fixture::new().await;
    let receipt = f.request().await;
    resolve(&f.pool, &f.windows, "window", &receipt.id, true, f.source())
        .await
        .unwrap();
    f.windows.close_source("window", &f.key).unwrap();
    let opened = f
        .windows
        .open_for_test("window", f.root.path())
        .await
        .unwrap();
    assert_ne!(opened.id.to_string(), f.key);
    assert_eq!(f.decision(&receipt.id).await, "allowed");
    assert!(
        resolve(&f.pool, &f.windows, "window", &receipt.id, true, f.source())
            .await
            .is_err()
    );
    let fresh = WindowSources::default();
    assert!(fresh
        .consume_write(&receipt.id, f.approval_source())
        .await
        .is_err());
}

#[tokio::test]
async fn equal_project_ids_in_two_physical_folders_do_not_share_decisions() {
    let f = Fixture::new().await;
    let original = f.request().await;
    let foreign_root = tempfile::tempdir().unwrap();
    let read = f
        .windows
        .capture_read("window", &f.key, &f.id, &f.commit)
        .await
        .unwrap();
    let mut project = read.project().clone();
    project.name = "Foreign same-ID folder".into();
    assert!(matches!(
        FolderProject::create(foreign_root.path(), project, vec![], None)
            .await
            .unwrap(),
        Publication::Committed(_)
    ));
    let opened = f
        .windows
        .open_for_test("window", foreign_root.path())
        .await
        .unwrap();
    let foreign_key = opened.id.to_string();
    let foreign_commit = opened.snapshot.commit_id().to_string();
    drop(opened);
    let foreign = || Source {
        project_id: f.id.clone(),
        source_id: foreign_key.clone(),
        commit_id: foreign_commit.clone(),
    };
    assert!(
        resolve(&f.pool, &f.windows, "window", &original.id, true, foreign())
            .await
            .is_err()
    );
    assert_eq!(f.decision(&original.id).await, "pending");
    let second = request(
        &f.pool,
        &f.windows,
        "window",
        Request {
            source_id: foreign_key.clone(),
            commit_id: foreign_commit.clone(),
            ..f.input()
        },
    )
    .await
    .unwrap();
    assert!(
        resolve(&f.pool, &f.windows, "window", &second.id, true, f.source())
            .await
            .is_err()
    );
    resolve(
        &f.pool,
        &f.windows,
        "window",
        &original.id,
        true,
        f.source(),
    )
    .await
    .unwrap();
    f.windows
        .consume_write(&original.id, f.approval_source())
        .await
        .unwrap();
    assert!(f
        .windows
        .consume_write(&second.id, f.approval_source())
        .await
        .is_err());
    resolve(&f.pool, &f.windows, "window", &second.id, true, foreign())
        .await
        .unwrap();
    f.windows
        .consume_write(
            &second.id,
            ApprovalSource {
                window: "window",
                project_id: &f.id,
                source_id: &foreign_key,
                commit_id: &foreign_commit,
            },
        )
        .await
        .unwrap();
    f.windows.close_source("window", &foreign_key).unwrap();
}

#[tokio::test]
async fn corrupt_server_settings_fail_closed_before_request_or_audit_creation() {
    let f = Fixture::new().await;
    sqlx::query("INSERT INTO settings(key,value) VALUES('confirmations','not-json')")
        .execute(&f.pool)
        .await
        .unwrap();
    assert!(request(&f.pool, &f.windows, "window", f.input())
        .await
        .is_err());
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM permissions")
        .fetch_one(&f.pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn audit_failure_cannot_allow_a_pending_request_and_cannot_undo_denial() {
    for allow in [true, false] {
        let f = Fixture::new().await;
        let before = f.bytes();
        let receipt = f.request().await;
        sqlx::query("DROP TABLE permissions")
            .execute(&f.pool)
            .await
            .unwrap();
        assert!(resolve(
            &f.pool,
            &f.windows,
            "window",
            &receipt.id,
            allow,
            f.source()
        )
        .await
        .is_err());
        assert!(f
            .windows
            .consume_write(&receipt.id, f.approval_source())
            .await
            .is_err());
        if !allow {
            assert!(f
                .windows
                .check_write_request(&receipt.id, f.approval_source())
                .is_err());
        }
        assert_eq!(before, f.bytes());
    }
}

#[tokio::test]
async fn forged_allowed_audit_row_is_not_authority() {
    let f = Fixture::new().await;
    let receipt = f.request().await;
    sqlx::query("UPDATE permissions SET decision='allowed' WHERE id=?")
        .bind(&receipt.id)
        .execute(&f.pool)
        .await
        .unwrap();
    assert!(
        resolve(&f.pool, &f.windows, "window", &receipt.id, true, f.source())
            .await
            .is_err()
    );
    assert!(f
        .windows
        .consume_write(&receipt.id, f.approval_source())
        .await
        .is_err());
}

#[tokio::test]
async fn concurrent_resolutions_allow_only_one_one_use_original_command() {
    let f = Fixture::new().await;
    let receipt = f.request().await;
    let (a, b) = tokio::join!(
        resolve(&f.pool, &f.windows, "window", &receipt.id, true, f.source()),
        resolve(&f.pool, &f.windows, "window", &receipt.id, true, f.source())
    );
    assert_ne!(a.is_ok(), b.is_ok());
    f.windows
        .consume_write(&receipt.id, f.approval_source())
        .await
        .unwrap();
    assert!(f
        .windows
        .consume_write(&receipt.id, f.approval_source())
        .await
        .is_err());
}

#[tokio::test]
async fn waiting_for_audit_sql_does_not_lock_source_registry_or_other_requests() {
    let f = Fixture::new().await;
    let receipt = f.request().await;
    let connection = f.pool.acquire().await.unwrap();
    let future = resolve(&f.pool, &f.windows, "window", &receipt.id, true, f.source());
    tokio::pin!(future);
    tokio::select! {
        result=&mut future=>panic!("audit unexpectedly completed: {result:?}"),
        _=tokio::time::sleep(Duration::from_millis(20))=>{},
    }
    f.windows
        .check_write_request(&receipt.id, f.approval_source())
        .unwrap();
    let second = tokio::time::timeout(
        Duration::from_secs(5),
        f.windows.request_write(
            f.approval_source(),
            WriteIntent::History(project_history::Direction::Undo),
        ),
    )
    .await
    .unwrap()
    .unwrap();
    f.windows
        .resolve_write(&second, "window", false)
        .await
        .unwrap();
    drop(connection);
    future.await.unwrap();
    f.windows
        .consume_write(&receipt.id, f.approval_source())
        .await
        .unwrap();
}

#[test]
fn folder_intents_reject_unknown_fields_and_replies_preserve_managed_wire_shape() {
    assert!(serde_json::from_value::<Intent>(serde_json::json!({"kind":"model","program":"program","prompt":"prompt","expectedRevision":null,"allow":true})).is_err());
    assert!(serde_json::from_value::<Intent>(
        serde_json::json!({"kind":"history","direction":"erase","expectedRevision":null})
    )
    .is_err());
    assert_eq!(
        serde_json::to_value(crate::permissions::PermissionReply::Managed(
            "legacy-id".into()
        ))
        .unwrap(),
        serde_json::json!("legacy-id")
    );
    assert_eq!(
        serde_json::to_value(Receipt {
            id: "request".into(),
            requires_confirmation: true
        })
        .unwrap(),
        serde_json::json!({"id":"request","requiresConfirmation":true})
    );
}
