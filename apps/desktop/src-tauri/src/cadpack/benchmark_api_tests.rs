use super::round_trip;
use crate::models::Project;

fn project() -> Project {
    let bytes = b"synthetic asset";
    let hash = crate::artifacts::digest(bytes);
    serde_json::from_value(serde_json::json!({
        "schemaVersion":1,"id":uuid::Uuid::new_v4().to_string(),"name":"CADPACK benchmark fixture",
        "units":"mm","agent":"codex","pinned":false,
        "createdAt":"2026-10-29T00:00:00Z","updatedAt":"2026-10-29T00:00:00Z",
        "currentRevision":null,"revisions":[],"messages":[],"exports":[],
        "files":[{"name":"first.step","kind":"attachment","size":bytes.len(),"sha256":hash},
                 {"name":"second.step","kind":"attachment","size":bytes.len(),"sha256":hash}]
    }))
    .unwrap()
}

#[test]
fn real_writer_reader_round_trip_proves_history_bytes_and_asset_deduplication() {
    let temp = tempfile::tempdir().unwrap();
    let project = project();
    let history = serde_json::to_vec(&serde_json::json!({"schemaVersion":1,
        "state":crate::project_history::HistoryState::from_project(&project).unwrap(),"events":[]}))
    .unwrap();
    let proof = round_trip(
        &project,
        &temp.path().join("model.cadpack"),
        Some(&history),
        |_| Ok(b"synthetic asset".to_vec()),
    )
    .unwrap();
    assert_eq!(
        proof.project_sha256,
        crate::artifacts::digest(&serde_json::to_vec(&project).unwrap())
    );
    assert_eq!(
        proof.history_sha256,
        Some(crate::artifacts::digest(&history))
    );
    assert_eq!(proof.attachment_references, 2);
    assert_eq!(proof.unique_assets, 1);
    assert_eq!(proof.unique_asset_bytes, b"synthetic asset".len() as u64);
    assert_eq!(proof.asset_reader_calls, 1);
    assert!(proof.archive_bytes > proof.unique_asset_bytes);
}

#[test]
fn wrong_source_checksum_preserves_previous_destination() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join("model.cadpack");
    std::fs::write(&target, b"previous destination").unwrap();
    let project = project();
    assert!(round_trip(&project, &target, None, |_| Ok(vec![b'x'; 15])).is_err());
    assert_eq!(std::fs::read(&target).unwrap(), b"previous destination");
    assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 1);
}

#[test]
fn declared_reference_budget_is_rejected_before_reading_assets_or_touching_destination() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join("model.cadpack");
    let mut project = project();
    project.files.clear();
    for (index, hash) in ['a', 'b', 'c'].into_iter().enumerate() {
        project.files.push(crate::models::ProjectFile {
            name: format!("asset-{index}.step"),
            kind: "attachment".into(),
            size: 40 * 1024 * 1024,
            data: None,
            sha256: Some(hash.to_string().repeat(64)),
        });
    }
    project.validate().unwrap();
    let mut calls = 0;
    assert!(round_trip(&project, &target, None, |_| {
        calls += 1;
        Ok(vec![])
    })
    .is_err());
    assert_eq!(calls, 0);
    assert!(!target.exists());
    assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 0);
}

#[test]
fn invalid_history_is_rejected_before_callback_and_publication() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join("model.cadpack");
    let mut calls = 0;
    assert!(round_trip(&project(), &target, Some(b"not history"), |_| {
        calls += 1;
        Ok(vec![])
    })
    .is_err());
    assert_eq!(calls, 0);
    assert!(!target.exists());
}
