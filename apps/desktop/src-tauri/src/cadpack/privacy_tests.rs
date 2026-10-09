use super::*;
use crate::project_history::{self, Direction};

fn fixture() -> Project {
    let id = uuid::Uuid::new_v4().to_string();
    let mut project: Project = serde_json::from_value(serde_json::json!({
        "schemaVersion":1,"id":id,"name":"Retained model name","units":"mm",
        "agent":"codex","pinned":false,"createdAt":"now","updatedAt":"now",
        "revisions":[],"currentRevision":null,"messages":[{
            "id":uuid::Uuid::new_v4().to_string(),"role":"user","text":"PRIVATE_CHAT_TOKEN","createdAt":"now"
        }],"files":[],"exports":[{"name":"C:/PRIVATE_EXPORT_TOKEN/output.step","createdAt":"now"}],
        "thumbnail":"data:image/jpeg;base64,UFJJVkFURV9USFVNQk5BSUxfVE9LRU4="
    })).unwrap();
    let bytes = b"saved geometry byte-for-byte";
    project.files.push(crate::models::ProjectFile {
        name: "part.step".into(),
        kind: "model".into(),
        size: bytes.len() as u64,
        sha256: Some(crate::artifacts::digest(bytes)),
        data: None,
    });
    let first = uuid::Uuid::new_v4().to_string();
    let mut ir: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../docs/fixtures/parameterized-disc.cad.json"
    ))
    .unwrap();
    for (revision_id, parent, value) in [
        (first.clone(), None, 8.0),
        (uuid::Uuid::new_v4().to_string(), Some(first), 12.0),
    ] {
        ir["revisionId"] = revision_id.clone().into();
        ir["parameters"][0]["valueMm"] = value.into();
        project.revisions.push(serde_json::from_value(serde_json::json!({
            "id":revision_id,"parent":parent,"createdAt":"now","prompt":"PRIVATE_PROMPT_TOKEN",
            "parameters":{"kind":"bracket","width":120,"depth":65,"height":60,"thickness":5,"holeDiameter":8,"holes":4},
            "source":"part.step","program":ir.to_string()
        })).unwrap());
        project.current_revision = Some(revision_id);
    }
    project.validate().unwrap();
    project
}

#[tokio::test]
async fn redacted_archive_preserves_history_geometry_and_excludes_conversation_metadata() {
    let temp = tempfile::tempdir().unwrap();
    let pool = crate::storage::open(&temp.path().join("privacy.sqlite"))
        .await
        .unwrap();
    let mut source = fixture();
    let private = b"PRIVATE_OPTIONAL_ATTACHMENT_TOKEN";
    source.files.push(crate::models::ProjectFile {
        name: "private-notes.pdf".into(),
        kind: "attachment".into(),
        size: private.len() as u64,
        sha256: Some(crate::artifacts::digest(private)),
        data: None,
    });
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("INSERT INTO projects(id,name,payload,updated_at) VALUES(?,?,?,?)")
        .bind(&source.id)
        .bind(&source.name)
        .bind(serde_json::to_string(&source).unwrap())
        .bind(&source.updated_at)
        .execute(&mut *tx)
        .await
        .unwrap();
    project_history::record_commit(&mut tx, None, &source, None)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let redacted = redaction::conversation_free(&source).unwrap();
    let history = project_history::export_history(&mut pool.acquire().await.unwrap(), &redacted)
        .await
        .unwrap();
    let path = temp.path().join("redacted.cadpack");
    write_bundle_with(&redacted, &path, Some(&history), |_| {
        Ok(b"saved geometry byte-for-byte".to_vec())
    })
    .unwrap();
    let bytes = std::fs::read(&path).unwrap();
    for secret in [
        "PRIVATE_CHAT_TOKEN",
        "PRIVATE_EXPORT_TOKEN",
        "PRIVATE_THUMBNAIL_TOKEN",
        "UFJJVkFURV9USFVNQk5BSUxfVE9LRU4=",
        "PRIVATE_PROMPT_TOKEN",
        "PRIVATE_OPTIONAL_ATTACHMENT_TOKEN",
    ] {
        assert!(!bytes
            .windows(secret.len())
            .any(|window| window == secret.as_bytes()));
    }
    let (restored, pending, archive) = read_bundle(&path).unwrap();
    assert_eq!(restored.name, source.name);
    assert_eq!(pending[0].1, b"saved geometry byte-for-byte");
    assert_eq!(restored.revisions[1].program, source.revisions[1].program);
    assert!(restored.messages.is_empty());
    assert!(restored.exports.is_empty());
    project_history::validate_archive(&restored, archive.as_ref().unwrap()).unwrap();
    let mut copied = restored.clone();
    copied.id = uuid::Uuid::new_v4().to_string();
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("INSERT INTO projects(id,name,payload,updated_at) VALUES(?,?,?,?)")
        .bind(&copied.id)
        .bind(&copied.name)
        .bind(serde_json::to_string(&copied).unwrap())
        .bind(&copied.updated_at)
        .execute(&mut *tx)
        .await
        .unwrap();
    project_history::import_history(&mut tx, &copied, archive.as_ref().unwrap())
        .await
        .unwrap();
    tx.commit().await.unwrap();
    for (direction, target) in [(Direction::Undo, 0), (Direction::Redo, 1)] {
        let state = project_history::load_state(&pool, &copied).await.unwrap();
        let (moved, intent) = project_history::prepare(
            &copied,
            &state,
            direction,
            copied.current_revision.as_deref(),
            &uuid::Uuid::new_v4().to_string(),
            "now",
        )
        .unwrap();
        assert_eq!(
            moved.revisions.last().unwrap().source,
            source.revisions[target].source
        );
        let program: serde_json::Value =
            serde_json::from_str(moved.revisions.last().unwrap().program.as_ref().unwrap())
                .unwrap();
        let original: serde_json::Value =
            serde_json::from_str(source.revisions[target].program.as_ref().unwrap()).unwrap();
        assert_eq!(program["parameters"], original["parameters"]);
        let mut tx = pool.begin().await.unwrap();
        sqlx::query("UPDATE projects SET payload=? WHERE id=?")
            .bind(serde_json::to_string(&moved).unwrap())
            .bind(&moved.id)
            .execute(&mut *tx)
            .await
            .unwrap();
        project_history::record_commit(&mut tx, Some(&copied), &moved, Some(&intent))
            .await
            .unwrap();
        tx.commit().await.unwrap();
        copied = moved;
    }
    // Source is immutable and full export retains conversation data by default.
    let full = temp.path().join("full.cadpack");
    write_bundle_with(&source, &full, None, |entry| {
        Ok(if entry.name == "private-notes.pdf" {
            private.to_vec()
        } else {
            b"saved geometry byte-for-byte".to_vec()
        })
    })
    .unwrap();
    assert_eq!(
        read_bundle(&full).unwrap().0.messages[0].text,
        "PRIVATE_CHAT_TOKEN"
    );
    assert_eq!(source.revisions[0].prompt, "PRIVATE_PROMPT_TOKEN");
}

#[test]
fn redaction_drops_unreferenced_native_attachments_but_retains_legacy_script_inputs() {
    let mut source = fixture();
    source.files.push(crate::models::ProjectFile {
        name: "private-notes.pdf".into(),
        kind: "attachment".into(),
        size: 7,
        sha256: Some(crate::artifacts::digest(b"private")),
        data: None,
    });
    assert_eq!(
        redaction::conversation_free(&source).unwrap().files.len(),
        1
    );
    source.revisions[0].program =
        Some("print('legacy code and input paths remain unchanged')".into());
    let redacted = redaction::conversation_free(&source).unwrap();
    assert_eq!(redacted.files.len(), 2);
    assert_eq!(redacted.revisions[0].program, source.revisions[0].program);
}

#[test]
fn redaction_retains_import_step_assets_referenced_only_by_old_revision_programs() {
    let mut source = fixture();
    let hash = crate::artifacts::digest(b"imported STEP geometry");
    source.files.push(crate::models::ProjectFile {
        name: "dependency.STP".into(),
        kind: "model".into(),
        size: 22,
        sha256: Some(hash.clone()),
        data: None,
    });
    let mut ir: serde_json::Value =
        serde_json::from_str(source.revisions[0].program.as_ref().unwrap()).unwrap();
    ir["features"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "id":"retained_asset", "name":"Imported geometry", "operation": {
                "type":"importStep", "assetId":format!("step_{hash}"), "sha256":hash
            }
        }));
    source.revisions[0].program = Some(ir.to_string());
    let redacted = redaction::conversation_free(&source).unwrap();
    assert_eq!(redacted.files.len(), 2);
    assert!(redacted
        .files
        .iter()
        .any(|file| file.name == "dependency.STP"));
}
