//! Explicit genuine-worker check of host policy -> decision -> retained CAD command.
use super::*;
use crate::agents::review_plan::{ExpectedCheck, ReviewPlan};
use crate::{
    cad_tasks::CadTaskGate,
    project_repository::sqlite_folder::api::{ModelChange, ModelRuntime, SavedCommit},
};
use std::{collections::HashMap, sync::atomic::AtomicBool};
use tokio::sync::Mutex;

fn model(width: f64) -> String {
    serde_json::json!({"schemaVersion":2,"revisionId":"permission_candidate","parameters":[],"features":[
        {"id":"profile","name":"Profile","operation":{"type":"rectangle","width":{"kind":"literal","mm":width},"depth":{"kind":"literal","mm":20}}},
        {"id":"pad","name":"Pad","operation":{"type":"extrude","sketchId":"profile","distance":{"kind":"literal","mm":10}}}
    ],"bodies":[{"id":"body","name":"Body","sourceFeatureId":"pad"}]}).to_string()
}
fn plan(volume: f64) -> ReviewPlan {
    ReviewPlan {
        assumptions: vec![],
        dimensions: vec![],
        affected_body_ids: vec!["body".into()],
        expected_checks: vec![ExpectedCheck::Volume {
            value_mm3: volume,
            tolerance_mm3: 1e-6,
        }],
    }
}
async fn apply(
    f: &Fixture,
    commit: &str,
    intent: Intent,
    required: bool,
    runtime: ModelRuntime<'_>,
) -> SavedCommit {
    let receipt = request(
        &f.pool,
        &f.windows,
        "window",
        Request {
            commit_id: commit.into(),
            intent,
            ..f.input()
        },
    )
    .await
    .unwrap();
    assert_eq!(receipt.requires_confirmation, required);
    let source = ApprovalSource {
        commit_id: commit,
        ..f.approval_source()
    };
    if required {
        assert!(f.windows.consume_write(&receipt.id, source).await.is_err());
        resolve(
            &f.pool,
            &f.windows,
            "window",
            &receipt.id,
            true,
            Source {
                commit_id: commit.into(),
                ..f.source()
            },
        )
        .await
        .unwrap();
    }
    let approved = f.windows.consume_write(&receipt.id, source).await.unwrap();
    assert!(f.windows.consume_write(&receipt.id, source).await.is_err());
    match approved.execute(runtime).await.unwrap() {
        ModelChange::Published(Publication::Committed(saved)) => saved,
        _ => panic!("actual original-command publication required"),
    }
}
async fn inspect(f: &Fixture, saved: &SavedCommit, width: f64, executable: &std::path::Path) {
    let read = f
        .windows
        .capture_read("window", &f.key, &f.id, saved.commit_id())
        .await
        .unwrap();
    let project = read.project();
    let revision = project
        .revisions
        .iter()
        .find(|revision| Some(&revision.id) == project.current_revision.as_ref())
        .unwrap();
    let file = project
        .files
        .iter()
        .find(|file| Some(&file.name) == revision.source.as_ref())
        .unwrap();
    let bytes = read.read_asset(file).await.unwrap();
    let stage = tempfile::tempdir().unwrap();
    std::fs::write(stage.path().join("source.step"), bytes).unwrap();
    let report = crate::native::inspection::inspect_with_executable(
        stage.path(),
        Arc::new(AtomicBool::new(false)),
        executable,
    )
    .await
    .unwrap();
    assert!((report.volume_mm3 - width * 20. * 10.).abs() < 1e-6);
    for (actual, expected) in report.bounds_mm.into_iter().zip([width, 20., 10.]) {
        assert!((actual - expected).abs() < 1e-6);
    }
    read.recheck().await.unwrap();
}

#[tokio::test]
#[ignore = "requires explicitly built genuine CAD worker"]
async fn real_permission_model_command_and_history() {
    let executable: std::path::PathBuf = std::env::var_os("FORMA_FOLDER_TEST_WORKER")
        .expect("explicit genuine worker")
        .into();
    let f = Fixture::new().await;
    let work = tempfile::tempdir().unwrap();
    let tasks = Mutex::new(HashMap::new());
    let gate = CadTaskGate::default();
    let runtime = || ModelRuntime {
        tasks: &tasks,
        gate: &gate,
        runtime_root: work.path(),
        transient_root: work.path(),
        executable: Some(&executable),
    };
    let first = apply(
        &f,
        &f.commit,
        Intent::Model {
            program: model(40.),
            prompt: "Original requested command".into(),
            expected_revision: None,
            review_plan: Some(plan(8000.)),
        },
        true,
        runtime(),
    )
    .await;
    inspect(&f, &first, 40., &executable).await;
    // A structurally valid plan with false geometry claims must survive approval
    // and be rejected by the actual common worker verifier, never silently dropped.
    let before = f.bytes();
    let receipt = request(
        &f.pool,
        &f.windows,
        "window",
        Request {
            commit_id: first.commit_id().into(),
            intent: Intent::Model {
                program: model(40.),
                prompt: "False metric claim".into(),
                expected_revision: first.project().current_revision.clone(),
                review_plan: Some(plan(1.)),
            },
            ..f.input()
        },
    )
    .await
    .unwrap();
    resolve(
        &f.pool,
        &f.windows,
        "window",
        &receipt.id,
        true,
        Source {
            commit_id: first.commit_id().into(),
            ..f.source()
        },
    )
    .await
    .unwrap();
    let approved = f
        .windows
        .consume_write(
            &receipt.id,
            ApprovalSource {
                commit_id: first.commit_id(),
                ..f.approval_source()
            },
        )
        .await
        .unwrap();
    let error = approved
        .execute(runtime())
        .await
        .err()
        .expect("false review plan was accepted");
    assert!(error.to_string().contains("EXPECTED_CHECK_FAILED"));
    assert_eq!(before, f.bytes());
    let second = apply(
        &f,
        first.commit_id(),
        Intent::Commands {
            commands: vec![crate::cad_ir::Command::SetLiteral {
                feature_id: "profile".into(),
                field: crate::cad_ir::DimensionField::Width,
                value_mm: 80.,
            }],
            prompt: "Original CAD command".into(),
            expected_revision: first.project().current_revision.clone().unwrap(),
        },
        true,
        runtime(),
    )
    .await;
    inspect(&f, &second, 80., &executable).await;
    let undo = apply(
        &f,
        second.commit_id(),
        Intent::History {
            direction: super::super::super::Direction::Undo,
            expected_revision: second.project().current_revision.clone(),
        },
        true,
        runtime(),
    )
    .await;
    inspect(&f, &undo, 40., &executable).await;
    sqlx::query("INSERT INTO settings(key,value) VALUES('confirmations',?)")
        .bind(r#"{"mode":"none","overrides":{}}"#)
        .execute(&f.pool)
        .await
        .unwrap();
    let redo = apply(
        &f,
        undo.commit_id(),
        Intent::History {
            direction: super::super::super::Direction::Redo,
            expected_revision: undo.project().current_revision.clone(),
        },
        false,
        runtime(),
    )
    .await;
    inspect(&f, &redo, 80., &executable).await;
    assert!(tasks.lock().await.is_empty());
    assert!(gate.status().unwrap().is_empty());
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM projects")
        .fetch_one(&f.pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}
