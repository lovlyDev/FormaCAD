use super::fixture::*;
use forma_core::{
    modeling::apply::apply_with_executable,
    project_history::{activity::activity_for, apply_history, Direction},
};

#[tokio::test]
async fn activity_lists_saved_model_events_in_reverse_commit_order() {
    let root = tempfile::tempdir().unwrap();
    let (state, baseline) = fixture(root.path()).await;
    let next = apply_with_executable(&state, request(&baseline, document(35.)), executable())
        .await
        .unwrap()
        .project;
    let undone = apply_history(
        &state,
        &next.id,
        next.current_revision.as_deref(),
        Direction::Undo,
        None,
    )
    .await
    .unwrap();
    let redone = apply_history(
        &state,
        &next.id,
        undone.current_revision.as_deref(),
        Direction::Redo,
        None,
    )
    .await
    .unwrap();
    let events = activity_for(&state, &next.id).await.unwrap();
    assert_eq!(
        events
            .iter()
            .map(|event| event.kind.as_str())
            .collect::<Vec<_>>(),
        ["redo", "undo", "edit", "edit"]
    );
    assert_eq!(events[0].to_revision, redone.current_revision);
    assert!(activity_for(&state, "../outside").await.is_err());
    assert!(activity_for(&state, &uuid::Uuid::new_v4().to_string())
        .await
        .unwrap()
        .is_empty());
    clean(&state).await;
}
