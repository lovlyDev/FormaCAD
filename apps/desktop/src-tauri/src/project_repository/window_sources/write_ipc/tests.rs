use super::*;
use serde_json::{json, Value};

fn request(source_id: String) -> Value {
    json!({
        "sourceId": source_id,
        "expectedCommitId": uuid::Uuid::new_v4().to_string(),
        "approvalId": uuid::Uuid::new_v4().to_string()
    })
}

#[test]
fn folder_write_requires_all_selectors_and_excludes_managed_payload() {
    let project_id = uuid::Uuid::new_v4().to_string();
    let valid = request(format!(
        "{}:1:{}",
        uuid::Uuid::new_v4(),
        uuid::Uuid::new_v4()
    ));
    let decoded: FolderWriteRequest = serde_json::from_value(valid.clone()).unwrap();
    assert!(decoded.validate(&project_id, false).is_ok());
    assert!(decoded.validate(&project_id, true).is_err());
    assert!(decoded.validate("invalid project", false).is_err());

    for key in ["sourceId", "expectedCommitId", "approvalId"] {
        let mut missing = valid.clone();
        missing.as_object_mut().unwrap().remove(key);
        assert!(serde_json::from_value::<FolderWriteRequest>(missing).is_err());

        let mut invalid = valid.clone();
        invalid[key] = json!("");
        let decoded: FolderWriteRequest = serde_json::from_value(invalid).unwrap();
        assert!(decoded.validate(&project_id, false).is_err());
    }

    let mut replacement = valid;
    replacement["program"] = json!("replacement program");
    assert!(serde_json::from_value::<FolderWriteRequest>(replacement).is_err());
}

#[cfg(any(
    windows,
    all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )
))]
#[tokio::test]
async fn folder_write_accepts_the_key_issued_by_the_registry() {
    use crate::project_repository::sqlite_folder::api::{
        FolderProject, FolderSources, Publication,
    };
    let root = tempfile::tempdir().unwrap();
    let project = fixture_project();
    let project_id = project.id.clone();
    assert!(matches!(
        FolderProject::create(root.path(), project, vec![], None)
            .await
            .unwrap(),
        Publication::Committed(_)
    ));
    let registry = FolderSources::default();
    let opened = registry.open(root.path()).await.unwrap();
    let mut payload = request(opened.id.to_string());
    payload["expectedCommitId"] = json!(opened.snapshot.commit_id());
    let decoded: FolderWriteRequest = serde_json::from_value(payload).unwrap();
    decoded.validate(&project_id, false).unwrap();
    assert!(registry.lookup(&decoded.source_id).is_ok());
}

#[test]
fn managed_reply_keeps_the_existing_project_json_shape() {
    let project = fixture_project();
    let expected = serde_json::to_value(&project).unwrap();
    assert_eq!(
        serde_json::to_value(WriteReply::from(project)).unwrap(),
        expected
    );
}

fn fixture_project() -> Project {
    serde_json::from_value(json!({
        "schemaVersion": 1, "id": uuid::Uuid::new_v4().to_string(),
        "name": "Managed", "units": "mm", "agent": "codex", "pinned": false,
        "createdAt": "2026-11-20T00:00:00Z", "updatedAt": "2026-11-20T00:00:00Z",
        "currentRevision": null, "messages": [], "exports": [], "files": [], "revisions": []
    }))
    .unwrap()
}
