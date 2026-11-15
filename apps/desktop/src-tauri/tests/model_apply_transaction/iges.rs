use super::fixture::*;
use forma_core::{
    cad_ir::Document,
    iges::{self, CommitRequest, ExportRequest, ImportRequest},
    native::{build_body_with_assets, Solid},
};
#[tokio::test]
async fn iges_round_trip_is_sealed_atomic_and_preserves_earlier_revision_and_attachments() {
    let root = tempfile::tempdir().unwrap();
    let (state, baseline) = fixture(root.path()).await;
    let before = snapshot(&state, &baseline.id).await;
    let export = ExportRequest {
        project_id: baseline.id.clone(),
        expected_revision: baseline.current_revision.clone().unwrap(),
        body_id: None,
    };
    let output = iges::export::prepare_with_executable(&state, &export, executable())
        .await
        .unwrap();
    assert!((output.report.source.volume_mm3 - 8000.).abs() < 1e-4);
    assert!(output.report.source.matches(&output.report.round_trip));
    assert_eq!(snapshot(&state, &baseline.id).await, before);
    let scoped = iges::export::prepare_with_executable(
        &state,
        &ExportRequest {
            project_id: baseline.id.clone(),
            expected_revision: export.expected_revision.clone(),
            body_id: Some("body".into()),
        },
        executable(),
    )
    .await
    .unwrap();
    assert!((scoped.report.round_trip.area_mm2 - 2800.).abs() < 1e-4);
    let input = ImportRequest {
        project_id: baseline.id.clone(),
        expected_revision: baseline.current_revision.clone(),
        name: "part.iges".into(),
        data: forma_core::artifacts::data_url("part.iges", &output.bytes),
    };
    let preview = iges::import::preview_with_executable(&state, &input, executable())
        .await
        .unwrap();
    let glb = forma_core::artifacts::decode_data(&preview.preview_data).unwrap();
    let length = u32::from_le_bytes(glb[12..16].try_into().unwrap()) as usize;
    let metadata: serde_json::Value = serde_json::from_slice(&glb[20..20 + length]).unwrap();
    assert!(metadata["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|node| node["name"] == "imported_body"));
    assert_eq!(snapshot(&state, &baseline.id).await, before);
    let invalid = ImportRequest {
        data: forma_core::artifacts::data_url("bad.igs", b"invalid IGES"),
        name: "bad.igs".into(),
        ..input.clone()
    };
    assert!(iges::import::commit_with_executable(
        &state,
        &CommitRequest {
            source_sha256: forma_core::artifacts::digest(b"invalid IGES"),
            prompt: "Import IGES".into(),
            input: invalid
        },
        executable()
    )
    .await
    .is_err());
    assert_eq!(snapshot(&state, &baseline.id).await, before);
    assert!(iges::import::commit_with_executable(
        &state,
        &CommitRequest {
            source_sha256: "0".repeat(64),
            prompt: "Import IGES".into(),
            input: input.clone()
        },
        executable()
    )
    .await
    .is_err());
    assert_eq!(snapshot(&state, &baseline.id).await, before);
    let result = iges::import::commit_with_executable(
        &state,
        &CommitRequest {
            source_sha256: preview.report.source_sha256,
            prompt: "Import IGES".into(),
            input,
        },
        executable(),
    )
    .await
    .unwrap();
    assert_eq!(result.project.revisions.len(), baseline.revisions.len() + 1);
    assert_eq!(
        result.project.revisions[0].program,
        baseline.revisions[0].program
    );
    let revision = result.project.revisions.last().unwrap();
    assert_eq!(revision.parent, baseline.current_revision);
    let doc: Document = serde_json::from_str(revision.program.as_deref().unwrap()).unwrap();
    assert_eq!(doc.revision_id, revision.id);
    assert_eq!(doc.bodies[0].id, "imported_body");
    let resolved = root.path().join("resolved-assets");
    std::fs::create_dir(&resolved).unwrap();
    forma_core::native::assets::stage_project_assets(&state, &result.project, &doc, &resolved)
        .unwrap();
    let solid = build_body_with_assets(&doc, "imported_body", Some(&resolved)).unwrap();
    assert!((solid.volume_mm3() - 8000.).abs() < 1e-4);
    assert!((solid.surface_area_mm2() - 2800.).abs() < 1e-4);
    let file = result
        .project
        .files
        .iter()
        .find(|f| Some(&f.name) == revision.source.as_ref())
        .unwrap();
    assert_eq!(file.sha256.as_ref(), Some(&result.report.output_sha256));
    let original = result
        .project
        .files
        .iter()
        .find(|f| f.name == "part.iges")
        .unwrap();
    assert_eq!(original.sha256.as_ref(), Some(&result.report.source_sha256));
    let path = root.path().join("canonical.step");
    std::fs::write(
        &path,
        forma_core::artifacts::read(&state, &baseline.id, file).unwrap(),
    )
    .unwrap();
    let restored = Solid::read_step(&path).unwrap();
    assert!((restored.volume_mm3() - 8000.).abs() < 1e-4);
    clean(&state).await;
}
