//! Synthetic input seams only: no geometry, worker or durable commit claim.
use super::*;

fn fixture() -> Project {
    serde_json::from_value(serde_json::json!({
        "schemaVersion":1,"id":"33333333-3333-4333-8333-333333333333",
        "name":"Source fixture","units":"mm","agent":"codex","pinned":false,
        "createdAt":"2026-10-01T00:00:00Z","updatedAt":"2026-10-01T00:00:00Z",
        "currentRevision":"11111111-1111-4111-8111-111111111111","messages":[],"exports":[],
        "files":[{"name":"base.step","size":8,"kind":"model","sha256":crate::artifacts::digest(b"old STEP")}],
        "revisions":[{"id":"11111111-1111-4111-8111-111111111111","parent":null,"createdAt":"2026-10-01T00:00:00Z",
            "prompt":"old revision","source":"base.step",
            "parameters":{"kind":"box","width":40,"depth":20,"height":10,"thickness":2,"holeDiameter":0,"holes":0}}]
    })).unwrap()
}
fn untyped() -> PreparedProgram {
    PreparedProgram {
        typed: false,
        compiled_legacy: None,
    }
}

#[test]
fn base_step_uses_captured_descriptor_callback_without_managed_storage() {
    let project = fixture();
    let before = serde_json::to_vec(&project).unwrap();
    let cwd = tempfile::tempdir().unwrap();
    let mut calls = 0;
    let mut reader = |file: &ProjectFile| {
        assert_eq!(file, &project.files[0]);
        calls += 1;
        Ok(b"old STEP".to_vec())
    };
    assert_eq!(
        stage_base(&project, &untyped(), cwd.path(), &mut reader)
            .unwrap()
            .as_deref(),
        Some("base.step")
    );
    assert_eq!(calls, 1);
    assert_eq!(
        std::fs::read(cwd.path().join("base.step")).unwrap(),
        b"old STEP"
    );
    assert_eq!(serde_json::to_vec(&project).unwrap(), before);
}

#[test]
fn typed_program_does_not_read_legacy_base() {
    let project = fixture();
    let cwd = tempfile::tempdir().unwrap();
    let mut reader =
        |_: &ProjectFile| -> Result<Vec<u8>> { panic!("typed input must not read legacy base") };
    let prepared = PreparedProgram {
        typed: true,
        compiled_legacy: None,
    };
    assert!(stage_base(&project, &prepared, cwd.path(), &mut reader)
        .unwrap()
        .is_none());
    assert_eq!(std::fs::read_dir(cwd.path()).unwrap().count(), 0);
}

#[test]
fn stale_base_source_error_is_preserved_and_materializes_nothing() {
    let project = fixture();
    let cwd = tempfile::tempdir().unwrap();
    let mut reader = |_: &ProjectFile| -> Result<Vec<u8>> {
        Err(AppError::Invalid("PROJECT_SNAPSHOT_CHANGED".into()))
    };
    let error = stage_base(&project, &untyped(), cwd.path(), &mut reader).unwrap_err();
    assert_eq!(error.to_string(), "PROJECT_SNAPSHOT_CHANGED");
    assert_eq!(std::fs::read_dir(cwd.path()).unwrap().count(), 0);
}

#[test]
fn missing_base_descriptor_is_rejected_before_callback() {
    let mut project = fixture();
    project.files.clear();
    let cwd = tempfile::tempdir().unwrap();
    let mut reader = |_: &ProjectFile| -> Result<Vec<u8>> { panic!("missing descriptor") };
    let error = stage_base(&project, &untyped(), cwd.path(), &mut reader).unwrap_err();
    assert_eq!(error.to_string(), "Base STEP is missing");
    assert_eq!(std::fs::read_dir(cwd.path()).unwrap().count(), 0);
}

#[cfg(feature = "native-worker-client")]
fn imported(hash: &str) -> Document {
    serde_json::from_value(serde_json::json!({
        "schemaVersion":2,"revisionId":"source_fixture","parameters":[],
        "features":[{"id":"imported","name":"Imported STEP","operation":{
            "type":"importStep","assetId":format!("step_{hash}"),"sha256":hash}}],
        "bodies":[{"id":"body","name":"Body","sourceFeatureId":"imported"}]
    }))
    .unwrap()
}

#[cfg(feature = "native-worker-client")]
#[test]
fn imported_step_uses_shared_validation_and_callback_without_managed_storage() {
    let project = fixture();
    let hash = project.files[0].sha256.as_deref().unwrap();
    let document = imported(hash);
    let cwd = tempfile::tempdir().unwrap();
    let before = serde_json::to_vec(&project).unwrap();
    let mut calls = 0;
    let mut reader = |file: &ProjectFile| {
        assert_eq!(file, &project.files[0]);
        calls += 1;
        Ok(b"old STEP".to_vec())
    };
    reader
        .stage_native(&project, &document, cwd.path())
        .unwrap();
    assert_eq!(calls, 1);
    assert_eq!(
        std::fs::read(cwd.path().join(crate::native::assets::relative_path(hash))).unwrap(),
        b"old STEP"
    );
    assert_eq!(serde_json::to_vec(&project).unwrap(), before);
}

#[cfg(feature = "native-worker-client")]
#[test]
fn imported_step_stale_error_is_not_reclassified_as_missing_or_checksum() {
    let project = fixture();
    let document = imported(project.files[0].sha256.as_deref().unwrap());
    let cwd = tempfile::tempdir().unwrap();
    let mut reader = |_: &ProjectFile| -> Result<Vec<u8>> {
        Err(AppError::Invalid("PROJECT_SNAPSHOT_CHANGED".into()))
    };
    let error = reader
        .stage_native(&project, &document, cwd.path())
        .unwrap_err();
    assert_eq!(error.to_string(), "PROJECT_SNAPSHOT_CHANGED");
    assert_eq!(std::fs::read_dir(cwd.path()).unwrap().count(), 0);
}

#[cfg(feature = "native-worker-client")]
#[test]
fn imported_step_checks_size_before_reader_and_checksum_before_materialization() {
    let mut project = fixture();
    let document = imported(project.files[0].sha256.as_deref().unwrap());
    let cwd = tempfile::tempdir().unwrap();
    project.files[0].size = crate::artifacts::MAX_FILE_BYTES as u64 + 1;
    let mut never = |_: &ProjectFile| -> Result<Vec<u8>> { panic!("oversized descriptor") };
    let error = never
        .stage_native(&project, &document, cwd.path())
        .unwrap_err();
    assert!(error.to_string().contains("ASSET_LIMIT"));
    project.files[0].size = 8;
    let mut corrupt = |_: &ProjectFile| -> Result<Vec<u8>> { Ok(b"bad STEP".to_vec()) };
    let error = corrupt
        .stage_native(&project, &document, cwd.path())
        .unwrap_err();
    assert!(error.to_string().contains("ASSET_CHECKSUM_MISMATCH"));
    assert_eq!(std::fs::read_dir(cwd.path()).unwrap().count(), 0);
}
