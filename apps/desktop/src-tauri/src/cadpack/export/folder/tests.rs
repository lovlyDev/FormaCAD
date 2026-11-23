use super::*;
use crate::{
    cadpack::{privacy_tests, read_bundle},
    models::{Project, ProjectFile},
    project_repository::sqlite_folder::api::{FolderProject, Publication},
};
use std::{fs, path::Path};

const PART: &[u8] = b"saved geometry byte-for-byte";
const INPUT: &[u8] = b"retained STEP fixture bytes";
const NOTE: &[u8] = b"private optional folder attachment";

async fn create_fixture(root: &Path) -> Project {
    let mut project = privacy_tests::fixture();
    project.thumbnail = None;
    project.exports.clear();
    let input_hash = crate::artifacts::digest(INPUT);
    let note_hash = crate::artifacts::digest(NOTE);
    for (name, kind, hash, bytes) in [
        ("dependency.STP", "model", &input_hash, INPUT),
        ("notes.txt", "attachment", &note_hash, NOTE),
    ] {
        project.files.push(ProjectFile {
            name: name.into(),
            kind: kind.into(),
            size: bytes.len() as u64,
            data: None,
            sha256: Some(hash.clone()),
        });
    }
    let revision = &mut project.revisions[0];
    let mut ir: serde_json::Value =
        serde_json::from_str(revision.program.as_ref().unwrap()).unwrap();
    ir["features"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "id": "retained_asset", "name": "Imported dependency", "operation": {
                "type": "importStep", "assetId": format!("step_{input_hash}"), "sha256": input_hash
            }
        }));
    revision.program = Some(ir.to_string());
    let assets = [PART, INPUT, NOTE]
        .into_iter()
        .map(|bytes| (crate::artifacts::digest(bytes), bytes.to_vec()))
        .collect();
    assert!(matches!(
        FolderProject::create(root, project.clone(), assets, None)
            .await
            .unwrap(),
        Publication::Committed(_)
    ));
    project
}

#[tokio::test]
async fn moved_folder_exports_full_and_share_bundles_without_writer_authority() {
    let outer = tempfile::tempdir().unwrap();
    let original = outer.path().join("old-location");
    fs::create_dir(&original).unwrap();
    let project = create_fixture(&original).await;
    let moved = outer.path().join("moved root");
    fs::rename(&original, &moved).unwrap();
    let windows = WindowSources::default();
    let opened = windows.open_for_test("main", &moved).await.unwrap();
    let key = opened.id.to_string();
    let commit = opened.snapshot.commit_id();
    let before = fs::read(moved.join("project.sqlite")).unwrap();
    let _writer = crate::project_access::folder_lease::FolderLease::acquire(&moved).unwrap();

    for redact in [false, true] {
        let operation = windows
            .capture_operation("main", &key, &project.id, commit)
            .await
            .unwrap();
        let path = outer.path().join(if redact {
            "share.cadpack"
        } else {
            "full.cadpack"
        });
        export(operation, path.clone(), redact).await.unwrap();
        let (restored, bytes, history) = read_bundle(&path).unwrap();
        assert_eq!(restored.current_revision, project.current_revision);
        assert_eq!(restored.revisions[0].program, project.revisions[0].program);
        assert_eq!(restored.messages.is_empty(), redact);
        assert_eq!(restored.revisions[0].prompt.is_empty(), redact);
        assert_eq!(restored.files.len(), if redact { 2 } else { 3 });
        assert!(bytes.iter().any(|(_, data)| data == INPUT));
        assert!(bytes.iter().any(|(_, data)| data == PART));
        assert_eq!(bytes.iter().any(|(_, data)| data == NOTE), !redact);
        crate::project_history::validate_archive(&restored, &history.unwrap()).unwrap();
        let archive = fs::read(&path).unwrap();
        assert!(!archive
            .windows(key.len())
            .any(|bytes| bytes == key.as_bytes()));
        assert_eq!(
            archive
                .windows(b"PRIVATE_CHAT_TOKEN".len())
                .any(|bytes| bytes == b"PRIVATE_CHAT_TOKEN"),
            !redact
        );
    }
    assert_eq!(fs::read(moved.join("project.sqlite")).unwrap(), before);
    assert!(!original.exists());
}

#[tokio::test]
async fn folder_export_rejects_source_destinations_and_closed_openings() {
    let root = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();
    let project = create_fixture(root.path()).await;
    let windows = WindowSources::default();
    let opened = windows.open_for_test("main", root.path()).await.unwrap();
    let key = opened.id.to_string();
    let commit = opened.snapshot.commit_id();
    let capture = || windows.capture_operation("main", &key, &project.id, commit);
    let inside = root.path().join("inside.cadpack");
    let result = export(capture().await.unwrap(), inside.clone(), false).await;
    assert_eq!(
        result.unwrap_err().to_string(),
        "PROJECT_BUNDLE_DESTINATION_INVALID"
    );
    assert!(!inside.exists());

    let selected = capture().await.unwrap();
    let outside = output.path().join("existing.cadpack");
    fs::write(&outside, b"existing output").unwrap();
    windows.close_window("main");
    assert_eq!(
        export(selected, outside.clone(), false)
            .await
            .unwrap_err()
            .to_string(),
        "PROJECT_SNAPSHOT_CHANGED"
    );
    assert_eq!(fs::read(outside).unwrap(), b"existing output");
}
