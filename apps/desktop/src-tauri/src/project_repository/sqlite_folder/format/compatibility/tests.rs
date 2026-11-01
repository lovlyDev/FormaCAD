use super::*;
use sqlx::sqlite::SqliteConnectOptions;

async fn fixture() -> (SqliteConnection, Snapshot) {
    let mut connection = SqliteConnection::connect_with(
        &SqliteConnectOptions::new()
            .in_memory(true)
            .foreign_keys(true),
    )
    .await
    .unwrap();
    let revision = uuid::Uuid::new_v4().to_string();
    let project: Project = serde_json::from_value(serde_json::json!({
        "schemaVersion":1,"id":uuid::Uuid::new_v4().to_string(),"name":"compatibility fixture",
        "units":"mm","agent":"codex","pinned":false,"createdAt":"2026-11-01T00:00:00Z",
        "updatedAt":"2026-11-01T00:00:00Z","currentRevision":revision,"messages":[],"exports":[],"files":[],
        "revisions":[{"id":revision,"parent":null,"createdAt":"2026-11-01T00:00:00Z","prompt":"synthetic model",
        "parameters":{"kind":"box","width":40,"depth":20,"height":10,"thickness":2,"holeDiameter":0,"holes":0}}]
    })).unwrap();
    let snapshot = committed(
        initialize(
            &mut connection,
            &uuid::Uuid::new_v4().to_string(),
            &project,
            None,
        )
        .await
        .unwrap(),
    );
    (connection, snapshot)
}
fn committed(result: CommitOutcome) -> Snapshot {
    match result {
        CommitOutcome::Committed(snapshot) => *snapshot,
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn metadata_continuation_preserves_latest_values_and_original_commit_seals() {
    let (mut connection, original) = fixture().await;
    let mut renamed = original.project().clone();
    renamed.name = "metadata saved while worker runs".into();
    let latest = committed(
        commit(&mut connection, &original, &renamed, None)
            .await
            .unwrap(),
    );
    let compatible = read(&mut connection, &original).await.unwrap();
    assert!(latest.same_saved_state(&compatible));
    assert_eq!(compatible.project().name, renamed.name);
    assert_ne!(
        compatible.expectation().commit_id(),
        original.expectation().commit_id()
    );
    assert_eq!(compatible.history(), original.history());
}

#[tokio::test]
async fn model_edit_and_history_moves_never_become_metadata_continuations() {
    let (mut connection, original) = fixture().await;
    let mut edited = original.project().clone();
    let mut revision = edited.revisions[0].clone();
    revision.id = uuid::Uuid::new_v4().to_string();
    revision.parent = edited.current_revision.clone();
    revision.parameters.width += 1.0;
    edited.current_revision = Some(revision.id.clone());
    edited.revisions.push(revision);
    let mut current = committed(
        commit(&mut connection, &original, &edited, None)
            .await
            .unwrap(),
    );
    assert!(read(&mut connection, &original).await.is_err());
    for direction in [
        crate::project_history::Direction::Undo,
        crate::project_history::Direction::Redo,
    ] {
        let (candidate, intent) = crate::project_history::prepare(
            current.project(),
            current.history_state(),
            direction,
            current.project().current_revision.as_deref(),
            &uuid::Uuid::new_v4().to_string(),
            "2026-11-01T01:00:00Z",
        )
        .unwrap();
        current = committed(
            commit(&mut connection, &current, &candidate, Some(&intent))
                .await
                .unwrap(),
        );
        assert!(read(&mut connection, &original).await.is_err());
    }
}
