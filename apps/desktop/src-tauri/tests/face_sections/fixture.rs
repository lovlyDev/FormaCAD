use forma_core::{
    cad_ir::Document,
    model_face_section::schema::{FaceSectionQuery, FaceSectionReport},
    model_measurement::schema::MeasurementBinding,
    native::{build_body_with_assets, face_section::client::section_with_executable},
};
use serde_json::json;
use std::{
    path::Path,
    sync::{atomic::AtomicBool, Arc},
};
pub(super) fn document() -> Document {
    serde_json::from_value(json!({"schemaVersion":2,"revisionId":uuid::Uuid::new_v4().to_string(),"parameters":[],"features":[
        {"id":"profile","name":"Box profile","operation":{"type":"rectangle","width":{"kind":"literal","mm":40},"depth":{"kind":"literal","mm":20}}},
        {"id":"pad","name":"Box pad","operation":{"type":"extrude","sketchId":"profile","distance":{"kind":"literal","mm":10}}},
        {"id":"circle","name":"Circle","operation":{"type":"circle","radius":{"kind":"literal","mm":8}}},
        {"id":"cylinder","name":"Cylinder","operation":{"type":"extrude","sketchId":"circle","distance":{"kind":"literal","mm":15}}},
        {"id":"move","name":"Move","operation":{"type":"translate","bodyFeatureId":"cylinder","offsetMm":[100,0,0]}}
    ],"bodies":[{"id":"body","name":"Box","sourceFeatureId":"pad"},{"id":"round_body","name":"Cylinder","sourceFeatureId":"move"}]})).unwrap()
}
pub(super) fn staged() -> (tempfile::TempDir, MeasurementBinding) {
    stage(&document(), &[])
}
pub(super) fn stage(
    doc: &Document,
    imports: &[(String, Vec<u8>)],
) -> (tempfile::TempDir, MeasurementBinding) {
    let directory = tempfile::tempdir().unwrap();
    if !imports.is_empty() {
        std::fs::create_dir(directory.path().join("inputs")).unwrap();
    }
    for (hash, bytes) in imports {
        std::fs::write(
            directory
                .path()
                .join(forma_core::native::assets::relative_path(hash)),
            bytes,
        )
        .unwrap();
    }
    let raw = serde_json::to_vec(doc).unwrap();
    std::fs::write(directory.path().join("document.json"), &raw).unwrap();
    let mut bodies = doc.bodies.iter();
    let mut whole =
        build_body_with_assets(doc, &bodies.next().unwrap().id, Some(directory.path())).unwrap();
    for body in bodies {
        whole = whole
            .compound(&build_body_with_assets(doc, &body.id, Some(directory.path())).unwrap())
            .unwrap();
    }
    whole
        .write_step(&directory.path().join("source.step"))
        .unwrap();
    let source = std::fs::read(directory.path().join("source.step")).unwrap();
    let seals =
        forma_core::native::reference_measurements::execute::imported_seals(doc, directory.path())
            .unwrap();
    (
        directory,
        MeasurementBinding {
            revision_id: doc.revision_id.clone(),
            body_id: "body".into(),
            source_sha256: forma_core::artifacts::digest(&source),
            source_size: source.len() as u64,
            document_sha256: forma_core::artifacts::digest(&raw),
            imported_asset_seals: seals,
        },
    )
}
pub(super) fn query(role: &str, offset: f64) -> FaceSectionQuery {
    serde_json::from_value(json!({"reference":{"schemaVersion":1,"kind":"face","ownerFeatureId":"pad","role":role,"occurrencePath":[]},"offsetMm":offset,"deflectionMm":0.01})).unwrap()
}
pub(super) async fn section_result(
    path: &Path,
    binding: &MeasurementBinding,
    query: &FaceSectionQuery,
) -> forma_core::core::Result<FaceSectionReport> {
    section_with_executable(
        path,
        Arc::new(AtomicBool::new(false)),
        Path::new(env!("CARGO_BIN_EXE_forma-cad-worker")),
        binding,
        query,
    )
    .await
}
pub(super) async fn section(
    path: &Path,
    binding: &MeasurementBinding,
    query: &FaceSectionQuery,
) -> FaceSectionReport {
    section_result(path, binding, query).await.unwrap()
}
