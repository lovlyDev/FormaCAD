//! Pure candidate tests. These byte strings are not CAD/kernel fixtures.
use super::*;

const OLD: &str = "11111111-1111-4111-8111-111111111111";
const NEXT: &str = "22222222-2222-4222-8222-222222222222";
const NOW: &str = "2026-10-31T12:34:56Z";

fn project() -> Project {
    let value: Project = serde_json::from_value(serde_json::json!({
        "schemaVersion":1,"id":"33333333-3333-4333-8333-333333333333",
        "name":"Prepared fixture","units":"mm","agent":"codex","pinned":true,
        "createdAt":"2026-10-01T00:00:00Z","updatedAt":"2026-10-01T00:00:00Z",
        "thumbnail":"data:image/jpeg;base64,eA==","thumbnailRevision":OLD,
        "currentRevision":OLD,"messages":[{"id":"message","role":"assistant","text":"preserved metadata","createdAt":"2026-10-01T00:00:00Z"}],
        "exports":[{"name":"old.step","createdAt":"2026-10-01T00:00:00Z"}],
        "files":[{"name":"base.step","size":8,"kind":"model","sha256":crate::artifacts::digest(b"old STEP")}],
        "revisions":[{"id":OLD,"parent":null,"createdAt":"2026-10-01T00:00:00Z",
            "prompt":"old revision","source":"base.step",
            "parameters":{"kind":"box","width":40,"depth":20,"height":10,"thickness":2,"holeDiameter":0,"holes":0}}]
    })).unwrap();
    value.validate().unwrap();
    value
}
fn model() -> GeneratedModel {
    GeneratedModel {
        step: b"synthetic STEP bytes".to_vec(),
        preview: b"synthetic GLB bytes".to_vec(),
        program_base: Some("base.step".into()),
    }
}
fn prepare_fixture(source: Project, cancel: bool) -> Result<PreparedModelChange> {
    prepare(
        source,
        model(),
        NEXT.into(),
        "saved program".into(),
        "new prompt".into(),
        NOW.into(),
        &AtomicBool::new(cancel),
    )
}

#[test]
fn candidate_preserves_source_metadata_and_history_references_with_normalized_outputs() {
    let original = project();
    let before = serde_json::to_vec(&original).unwrap();
    let (next, pending) = prepare_fixture(original.clone(), false)
        .unwrap()
        .into_parts();
    assert_eq!(serde_json::to_vec(&original).unwrap(), before);
    assert_eq!(next.id, original.id);
    assert_eq!(next.name, original.name);
    assert_eq!(next.units, original.units);
    assert_eq!(next.agent, original.agent);
    assert_eq!(next.pinned, original.pinned);
    assert_eq!(next.created_at, original.created_at);
    assert_eq!(next.thumbnail, original.thumbnail);
    assert_eq!(next.thumbnail_revision, original.thumbnail_revision);
    assert_eq!(
        serde_json::to_value(&next.messages).unwrap(),
        serde_json::to_value(&original.messages).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&next.exports).unwrap(),
        serde_json::to_value(&original.exports).unwrap()
    );
    assert!(next.revisions.starts_with(&original.revisions));
    assert!(next.files.starts_with(&original.files));
    assert_eq!(next.revisions.len(), original.revisions.len() + 1);
    assert_eq!(next.current_revision.as_deref(), Some(NEXT));
    assert_eq!(next.updated_at, NOW);
    let revision = next.revisions.last().unwrap();
    assert_eq!(revision.id, NEXT);
    assert_eq!(revision.parent.as_deref(), Some(OLD));
    assert_eq!(revision.program.as_deref(), Some("saved program"));
    assert_eq!(revision.program_base.as_deref(), Some("base.step"));
    assert_eq!(revision.prompt, "new prompt");
    assert_eq!(revision.created_at, NOW);
    assert_eq!(
        revision.source.as_deref(),
        Some(format!("model-{NEXT}.step").as_str())
    );
    assert_eq!(
        revision.preview.as_deref(),
        Some(format!("preview-{NEXT}.glb").as_str())
    );
    assert_eq!(revision.parameters.kind, "blank");
    assert_eq!(
        (
            revision.parameters.width,
            revision.parameters.depth,
            revision.parameters.height
        ),
        (120.0, 65.0, 60.0)
    );
    assert_eq!(
        (
            revision.parameters.thickness,
            revision.parameters.hole_diameter,
            revision.parameters.holes
        ),
        (5.0, 8.0, 4)
    );
    assert_eq!(pending.len(), 2);
    for (file, bytes) in next.files.iter().skip(original.files.len()).zip([
        b"synthetic STEP bytes".as_slice(),
        b"synthetic GLB bytes".as_slice(),
    ]) {
        assert!(file.data.is_none());
        assert_eq!(file.kind, "model");
        assert_eq!(file.size, bytes.len() as u64);
        assert_eq!(
            file.sha256.as_deref(),
            Some(crate::artifacts::digest(bytes).as_str())
        );
        assert!(pending
            .iter()
            .any(|(hash, data)| Some(hash) == file.sha256.as_ref() && data == bytes));
    }
    next.validate().unwrap();
}

#[test]
fn identical_outputs_share_content_address_without_changing_pending_contract() {
    let bytes = b"same synthetic output".to_vec();
    let generated = GeneratedModel {
        step: bytes.clone(),
        preview: bytes.clone(),
        program_base: None,
    };
    let (next, pending) = prepare(
        project(),
        generated,
        NEXT.into(),
        "program".into(),
        "prompt".into(),
        NOW.into(),
        &AtomicBool::new(false),
    )
    .unwrap()
    .into_parts();
    let outputs = &next.files[next.files.len() - 2..];
    assert_eq!(outputs[0].sha256, outputs[1].sha256);
    // normalize intentionally returns both supplied blobs, as before extraction.
    // CAS publication may deduplicate this key; this pure function never writes CAS.
    assert_eq!(pending.len(), 2);
    assert!(pending
        .iter()
        .all(|(hash, data)| hash == &crate::artifacts::digest(&bytes) && data == &bytes));
}

#[test]
fn cancelled_preparation_returns_no_candidate_and_does_not_mutate_source() {
    let source = project();
    let before = serde_json::to_vec(&source).unwrap();
    assert_eq!(
        prepare_fixture(source.clone(), true)
            .err()
            .unwrap()
            .to_string(),
        "CAD_TASK_CANCELLED"
    );
    assert_eq!(serde_json::to_vec(&source).unwrap(), before);
}

#[test]
fn invalid_duplicate_revision_is_rejected_before_candidate_success() {
    let source = project();
    let result = prepare(
        source,
        model(),
        OLD.into(),
        "program".into(),
        "prompt".into(),
        NOW.into(),
        &AtomicBool::new(false),
    );
    assert_eq!(result.err().unwrap().to_string(), "Duplicate revision ID");
}

#[test]
fn invalid_project_metadata_is_rejected_before_candidate_success() {
    let mut source = project();
    source.name.clear();
    assert!(prepare_fixture(source, false).is_err());
}

#[test]
fn normalization_failure_still_precedes_cancellation() {
    let mut generated = model();
    generated.step.clear();
    let result = prepare(
        project(),
        generated,
        NEXT.into(),
        "program".into(),
        "prompt".into(),
        NOW.into(),
        &AtomicBool::new(true),
    );
    assert_eq!(
        result.err().unwrap().to_string(),
        "Attachment is empty or too large"
    );
}
