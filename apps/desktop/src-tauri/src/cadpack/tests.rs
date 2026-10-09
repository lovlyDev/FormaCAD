use super::*;
use crate::models::ProjectFile;
fn fixture_project() -> Project {
    Project {
        schema_version: 1,
        id: uuid::Uuid::new_v4().to_string(),
        name: "Portable part".into(),
        units: "mm".into(),
        agent: "codex".into(),
        pinned: false,
        thumbnail: None,
        thumbnail_revision: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        revisions: vec![],
        current_revision: None,
        messages: vec![],
        files: vec![],
        exports: vec![],
    }
}
fn fixture_bundle(path: &Path, tamper: bool, extra: Option<&str>) {
    let mut project = fixture_project();
    let hash = crate::artifacts::digest(b"solid");
    project.files.push(ProjectFile {
        name: "part.step".into(),
        size: 5,
        kind: "model".into(),
        data: None,
        sha256: Some(hash.clone()),
    });
    let mut zip = ZipWriter::new(File::create(path).unwrap());
    let options = SimpleFileOptions::default();
    zip.start_file("manifest.json", options).unwrap();
    zip.write_all(
        &serde_json::to_vec(&Manifest {
            format: "Forma CAD project".into(),
            format_version: 1,
            project_id: project.id.clone(),
            history: None,
        })
        .unwrap(),
    )
    .unwrap();
    zip.start_file("project.json", options).unwrap();
    zip.write_all(&serde_json::to_vec(&project).unwrap())
        .unwrap();
    zip.start_file(attachment_path(&hash), options).unwrap();
    zip.write_all(if tamper { b"wrong" } else { b"solid" })
        .unwrap();
    if let Some(name) = extra {
        zip.start_file(name, options).unwrap();
        zip.write_all(b"malicious").unwrap();
    }
    zip.finish().unwrap();
}
#[test]
fn entry_reader_rejects_oversized_data() {
    assert!(read_limited(&mut &b"12345"[..], 4).is_err());
}
#[test]
fn attachment_names_are_hash_only() {
    assert!(crate::artifacts::valid_digest(&"a".repeat(64)));
    assert!(!crate::artifacts::valid_digest("../project.json"));
    assert_eq!(attachment_path(&"a".repeat(64)).len(), 76);
}
#[test]
fn bundle_round_trip_validates_attachment_integrity() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("part.cadpack");
    fixture_bundle(&path, false, None);
    let (project, pending, history) = read_bundle(&path).unwrap();
    assert_eq!(project.name, "Portable part");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].1, b"solid");
    assert!(history.is_none());
    fixture_bundle(&path, true, None);
    assert!(read_bundle(&path).is_err());
}
#[test]
fn bundle_rejects_unexpected_and_traversal_entries() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("part.cadpack");
    fixture_bundle(&path, false, Some("../escape.txt"));
    assert!(read_bundle(&path).is_err());
    fixture_bundle(&path, false, Some("attachments/extra"));
    assert!(read_bundle(&path).is_err());
}

#[test]
fn version_two_history_requires_matching_descriptor_and_verified_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("history.cadpack");
    for variant in 0..6 {
        let project = fixture_project();
        let bytes = serde_json::to_vec(&serde_json::json!({
            "schemaVersion": 1,
            "state": {"entries": [null], "cursor": 0, "head": null, "generation": 0},
            "events": []
        }))
        .unwrap();
        let mut descriptor = serde_json::to_value(history::descriptor(&bytes).unwrap()).unwrap();
        match variant {
            1 => descriptor["size"] = 1.into(),
            2 => descriptor["sha256"] = "a".repeat(64).into(),
            _ => (),
        }
        let mut zip = ZipWriter::new(File::create(&path).unwrap());
        let options = SimpleFileOptions::default();
        zip.start_file("manifest.json", options).unwrap();
        zip.write_all(
            &serde_json::to_vec(&serde_json::json!({
                "format": "Forma CAD project",
                "formatVersion": if variant == 4 {1} else {2},
                "projectId": project.id,
                "history": if variant == 5 {serde_json::Value::Null} else {descriptor}
            }))
            .unwrap(),
        )
        .unwrap();
        zip.start_file("project.json", options).unwrap();
        zip.write_all(&serde_json::to_vec(&project).unwrap())
            .unwrap();
        if variant != 3 {
            zip.start_file("history.json", options).unwrap();
            zip.write_all(&bytes).unwrap();
        }
        zip.finish().unwrap();
        if variant == 0 {
            let (restored, files, history) = read_bundle(&path).unwrap();
            assert_eq!(restored.id, project.id);
            assert!(files.is_empty());
            assert_eq!(history, Some(bytes));
        } else {
            assert!(read_bundle(&path).is_err(), "variant {variant}");
        }
    }
}
