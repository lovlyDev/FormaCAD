use super::fixture::*;
use forma_core::{
    cad_ir::{Command, Document, Operation},
    modeling::{amend_transform, apply::apply_with_executable},
    project_history::{self, Direction},
};

fn matrix(x: f64) -> [f64; 12] {
    [1., 0., 0., x, 0., 1., 0., 0., 0., 0., 1., 0.]
}
fn document(project: &forma_core::models::Project) -> Document {
    serde_json::from_str(
        project
            .revisions
            .iter()
            .find(|revision| Some(&revision.id) == project.current_revision.as_ref())
            .unwrap()
            .program
            .as_ref()
            .unwrap(),
    )
    .unwrap()
}
#[tokio::test]
async fn actual_amend_replaces_one_undo_slot_preserves_artifacts_and_portable_history() {
    let root = tempfile::tempdir().unwrap();
    let (state, base) = fixture(root.path()).await;
    let request = forma_core::modeling::commands::prepare(
        &state,
        base.id.clone(),
        vec![Command::TransformBody {
            body_id: "body".into(),
            feature_id: "move".into(),
            name: "Move".into(),
            matrix: matrix(10.),
        }],
        "Move".into(),
        base.current_revision.clone().unwrap(),
    )
    .await
    .unwrap();
    let moved = apply_with_executable(&state, request, executable())
        .await
        .unwrap()
        .project;
    let first = amend_transform::status(&state, &base.id)
        .await
        .unwrap()
        .unwrap();
    let old_count = moved.revisions.len();
    let mut current = moved.clone();
    for x in [20., 30.] {
        let status = amend_transform::status(&state, &base.id)
            .await
            .unwrap()
            .unwrap();
        let request = amend_transform::prepare(
            &state,
            base.id.clone(),
            status.expected_revision,
            status.expected_generation,
            status.transaction_id,
            vec![amend_transform::TransformReplacement {
                command_index: 0,
                matrix: matrix(x),
            }],
        )
        .await
        .unwrap();
        current = apply_with_executable(&state, request, executable())
            .await
            .unwrap()
            .project;
        let doc = document(&current);
        assert_eq!(doc.features.len(), 3);
        assert!(
            matches!(&doc.features[2].operation, Operation::Transform { body_feature_id, matrix } if body_feature_id == "pad" && matrix[3] == x)
        );
        assert!(current.revisions.starts_with(&moved.revisions));
        let mut connection = state.pool.acquire().await.unwrap();
        let archive = project_history::export_history(&mut connection, &current)
            .await
            .unwrap();
        project_history::validate_archive(&current, &archive).unwrap();
    }
    assert_eq!(current.revisions.len(), old_count + 2);
    let before = snapshot(&state, &base.id).await;
    assert_eq!(
        amend_transform::prepare(
            &state,
            base.id.clone(),
            first.expected_revision,
            first.expected_generation,
            first.transaction_id,
            vec![amend_transform::TransformReplacement {
                command_index: 0,
                matrix: matrix(99.)
            }]
        )
        .await
        .err()
        .unwrap()
        .to_string(),
        "AMEND_STALE"
    );
    assert_eq!(snapshot(&state, &base.id).await, before);
    let undone = project_history::apply_history(
        &state,
        &base.id,
        current.current_revision.as_deref(),
        Direction::Undo,
        None,
    )
    .await
    .unwrap();
    assert_eq!(document(&undone).features, document(&base).features);
    assert_eq!(
        undone.revisions.last().unwrap().source,
        base.revisions.last().unwrap().source
    );
    assert!(amend_transform::status(&state, &base.id)
        .await
        .unwrap()
        .is_none());
    let redone = project_history::apply_history(
        &state,
        &base.id,
        undone.current_revision.as_deref(),
        Direction::Redo,
        None,
    )
    .await
    .unwrap();
    assert_eq!(document(&redone).features, document(&current).features);
    assert!(amend_transform::status(&state, &base.id)
        .await
        .unwrap()
        .is_none());
    // A fresh transform after history travel is amendable against the logical base snapshot.
    let request = forma_core::modeling::commands::prepare(
        &state,
        base.id.clone(),
        vec![Command::TransformBody {
            body_id: "body".into(),
            feature_id: "again".into(),
            name: "Move again".into(),
            matrix: matrix(5.),
        }],
        "Move again".into(),
        redone.current_revision.clone().unwrap(),
    )
    .await
    .unwrap();
    let branched = apply_with_executable(&state, request, executable())
        .await
        .unwrap()
        .project;
    let status = amend_transform::status(&state, &base.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        status.base_revision,
        current.current_revision.clone().unwrap()
    );
    let request = amend_transform::prepare(
        &state,
        base.id.clone(),
        status.expected_revision,
        status.expected_generation,
        status.transaction_id,
        vec![amend_transform::TransformReplacement {
            command_index: 0,
            matrix: matrix(7.),
        }],
    )
    .await
    .unwrap();
    let adjusted = apply_with_executable(&state, request, executable())
        .await
        .unwrap()
        .project;
    assert!(adjusted.revisions.starts_with(&branched.revisions));
    let previous = project_history::apply_history(
        &state,
        &base.id,
        adjusted.current_revision.as_deref(),
        Direction::Undo,
        None,
    )
    .await
    .unwrap();
    assert_eq!(document(&previous).features, document(&redone).features);
    clean(&state).await;
}

#[tokio::test]
async fn amend_stale_permissions_and_worker_failures_leave_saved_project_unchanged() {
    let root = tempfile::tempdir().unwrap();
    let (state, base) = fixture(root.path()).await;
    let request = forma_core::modeling::commands::prepare(
        &state,
        base.id.clone(),
        vec![Command::TransformBody {
            body_id: "body".into(),
            feature_id: "move".into(),
            name: "Move".into(),
            matrix: matrix(10.),
        }],
        "Move".into(),
        base.current_revision.clone().unwrap(),
    )
    .await
    .unwrap();
    let moved = apply_with_executable(&state, request, executable())
        .await
        .unwrap()
        .project;
    let status = amend_transform::status(&state, &base.id)
        .await
        .unwrap()
        .unwrap();
    let before = snapshot(&state, &base.id).await;
    assert!(amend_transform::prepare(
        &state,
        base.id.clone(),
        status.expected_revision.clone(),
        status.expected_generation + 1,
        status.transaction_id.clone(),
        vec![amend_transform::TransformReplacement {
            command_index: 0,
            matrix: matrix(20.)
        }]
    )
    .await
    .is_err());
    assert_eq!(snapshot(&state, &base.id).await, before);
    let request = amend_transform::prepare(
        &state,
        base.id.clone(),
        status.expected_revision.clone(),
        status.expected_generation,
        status.transaction_id.clone(),
        vec![amend_transform::TransformReplacement {
            command_index: 0,
            matrix: matrix(20.),
        }],
    )
    .await
    .unwrap();
    assert!(
        apply_with_executable(&state, request, &root.path().join("missing-worker.exe"))
            .await
            .is_err()
    );
    assert_eq!(snapshot(&state, &base.id).await, before);
    clean(&state).await;
    sqlx::query("DELETE FROM settings WHERE key='confirmations'")
        .execute(&state.pool)
        .await
        .unwrap();
    let request = amend_transform::prepare(
        &state,
        base.id.clone(),
        status.expected_revision,
        status.expected_generation,
        status.transaction_id,
        vec![amend_transform::TransformReplacement {
            command_index: 0,
            matrix: matrix(20.),
        }],
    )
    .await
    .unwrap();
    assert!(matches!(
        apply_with_executable(&state, request, executable()).await,
        Err(forma_core::core::AppError::Permission(_))
    ));
    assert_eq!(snapshot(&state, &base.id).await, before);
    assert_eq!(
        forma_core::projects::get(&state, &base.id)
            .await
            .unwrap()
            .current_revision,
        moved.current_revision
    );
}
