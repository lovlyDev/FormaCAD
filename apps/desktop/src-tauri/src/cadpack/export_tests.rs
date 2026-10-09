use super::*;
use crate::{core::AppState, models::Project};
use std::path::Path;

async fn fixture(root: &Path) -> (AppState, Project) {
    let pool = crate::storage::open(&root.join("export.sqlite"))
        .await
        .unwrap();
    let (_, guard) = tracing_appender::non_blocking(std::io::sink());
    let state = AppState {
        root: root.into(),
        pool,
        cad_tasks: Default::default(),
        project_access: Default::default(),
        grants: Default::default(),
        tasks: Default::default(),
        writes: Default::default(),
        _log_guard: guard,
    };
    let revision = uuid::Uuid::new_v4().to_string();
    let bytes = b"synthetic sealed STEP attachment, not a geometry test";
    let project: Project = serde_json::from_value(serde_json::json!({
        "schemaVersion":1,"id":uuid::Uuid::new_v4().to_string(),"name":"Snapshot export",
        "units":"mm","agent":"codex","pinned":false,"createdAt":"now","updatedAt":"now",
        "currentRevision":revision,"revisions":[{
            "id":revision,"parent":null,"createdAt":"now","prompt":"PRIVATE_PROMPT",
            "parameters":{"kind":"bracket","width":40,"depth":20,"height":10,"thickness":2,"holeDiameter":4,"holes":4},
            "source":"part.step"
        }],"messages":[{"id":uuid::Uuid::new_v4().to_string(),"role":"user","text":"PRIVATE_CONVERSATION","createdAt":"now"}],
        "files":[{"name":"part.step","kind":"model","size":bytes.len(),"sha256":crate::artifacts::digest(bytes),"data":crate::artifacts::data_url("part.step",bytes)}],"exports":[]
    })).unwrap();
    let mut project = project;
    let pending = crate::artifacts::normalize(&mut project).unwrap();
    let project = crate::projects::persist(&state, project, pending, None)
        .await
        .unwrap();
    (state, project)
}

#[tokio::test]
async fn full_and_redacted_exports_preserve_the_same_committed_history_and_geometry() {
    let temp = tempfile::tempdir().unwrap();
    let (state, source) = fixture(temp.path()).await;
    let before = std::fs::read(temp.path().join(&source.id).join("manifest.json")).unwrap();
    let snapshot = legacy::capture_with_history(&state, &source.id)
        .await
        .unwrap();
    for redact in [false, true] {
        let destination = temp.path().join(if redact {
            "redacted.cadpack"
        } else {
            "full.cadpack"
        });
        write_snapshot(&state, &source.id, &destination, redact)
            .await
            .unwrap();
        let (restored, attachments, history) = super::super::read_bundle(&destination).unwrap();
        assert_eq!(restored.id, source.id);
        assert_eq!(restored.current_revision, source.current_revision);
        assert_eq!(history, snapshot.history);
        assert_eq!(
            attachments[0].1,
            b"synthetic sealed STEP attachment, not a geometry test"
        );
        assert_eq!(restored.messages.is_empty(), redact);
        assert_eq!(restored.revisions[0].prompt.is_empty(), redact);
        let archive = std::fs::read(&destination).unwrap();
        assert_eq!(
            archive
                .windows(b"PRIVATE_CONVERSATION".len())
                .any(|part| part == b"PRIVATE_CONVERSATION"),
            !redact
        );
    }
    assert_eq!(
        std::fs::read(temp.path().join(&source.id).join("manifest.json")).unwrap(),
        before
    );
    assert_eq!(
        state
            .project_access
            .status(&state.root, &source.id)
            .unwrap()
            .mode,
        "closed"
    );
}

#[tokio::test]
async fn pending_recovery_export_never_publishes_source_or_changes_existing_destination() {
    let temp = tempfile::tempdir().unwrap();
    let (state, source) = fixture(temp.path()).await;
    let root = temp.path().join(&source.id);
    let before = std::fs::read(root.join("manifest.json")).unwrap();
    sqlx::query("INSERT INTO project_storage_commits(project_id,base_manifest_sha256,target_manifest,project_sha256) VALUES(?,?,?,?)")
        .bind(&source.id).bind(crate::artifacts::digest(&before)).bind(String::from_utf8(before.clone()).unwrap())
        .bind(crate::artifacts::digest(&serde_json::to_vec(&source).unwrap())).execute(&state.pool).await.unwrap();
    let destination = temp.path().join("existing.cadpack");
    std::fs::write(&destination, b"existing export").unwrap();
    let error = write_snapshot(&state, &source.id, &destination, false)
        .await
        .unwrap_err();
    assert!(error
        .to_string()
        .contains("PROJECT_SNAPSHOT_RECOVERY_REQUIRED"));
    assert_eq!(std::fs::read(&destination).unwrap(), b"existing export");
    assert_eq!(std::fs::read(root.join("manifest.json")).unwrap(), before);
    let pending: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM project_storage_commits WHERE project_id=?")
            .bind(&source.id)
            .fetch_one(&state.pool)
            .await
            .unwrap();
    assert_eq!(pending, 1);
    assert_eq!(
        state
            .project_access
            .status(&state.root, &source.id)
            .unwrap()
            .mode,
        "closed"
    );
}

#[tokio::test]
async fn altered_materialized_attachment_does_not_replace_an_existing_export() {
    let temp = tempfile::tempdir().unwrap();
    let (state, source) = fixture(temp.path()).await;
    let file = &source.files[0];
    let attachment = temp
        .path()
        .join(&source.id)
        .join("attachments")
        .join(format!("{}.step", file.sha256.as_deref().unwrap()));
    let tampered = vec![b'x'; file.size as usize];
    std::fs::write(&attachment, &tampered).unwrap();
    let destination = temp.path().join("existing.cadpack");
    std::fs::write(&destination, b"existing export").unwrap();
    assert!(write_snapshot(&state, &source.id, &destination, false)
        .await
        .is_err());
    assert_eq!(std::fs::read(&destination).unwrap(), b"existing export");
    assert_eq!(std::fs::read(&attachment).unwrap(), tampered);
}
