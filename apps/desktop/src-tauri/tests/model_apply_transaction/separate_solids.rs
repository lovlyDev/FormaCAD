use super::fixture::*;
use forma_core::{
    cad_ir::{Command, Document, SeparateBody},
    modeling::{apply::apply_with_executable, commands::prepare},
    native::{build_body, Solid},
};

#[tokio::test]
async fn split_preserves_geometry_and_rejects_a_stale_solid_count() {
    let root = tempfile::tempdir().unwrap();
    let (state, baseline) = fixture(root.path()).await;
    let mut compound = document(40.);
    compound["features"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "id":"pattern","name":"Pattern","operation":{
                "type":"patternLinear","bodyFeatureId":"pad","repeatHole":false,
                "layout":{"countU":2,"stepUMm":[50.,0.,0.],"countV":1,"stepVMm":[0.,0.,0.]}
            }
        }));
    compound["bodies"][0]["sourceFeatureId"] = "pattern".into();
    let source = apply_with_executable(&state, request(&baseline, compound), executable())
        .await
        .unwrap()
        .project;
    let before = snapshot(&state, &source.id).await;
    let split = |ids: &[&str]| {
        prepare(
            &state,
            source.id.clone(),
            vec![Command::SeparateSolids {
                body_id: "body".into(),
                new_bodies: ids
                    .iter()
                    .map(|id| SeparateBody {
                        id: (*id).into(),
                        name: (*id).into(),
                    })
                    .collect(),
            }],
            "Separate solids".into(),
            source.current_revision.clone().unwrap(),
        )
    };
    assert!(apply_with_executable(
        &state,
        split(&["part", "extra"]).await.unwrap(),
        executable()
    )
    .await
    .is_err());
    assert_eq!(snapshot(&state, &source.id).await, before);
    let next = apply_with_executable(&state, split(&["part"]).await.unwrap(), executable())
        .await
        .unwrap();
    assert!(next.committed);
    assert_eq!(next.project.revisions.len(), source.revisions.len() + 1);
    let revision = next.project.revisions.last().unwrap();
    let document: Document = serde_json::from_str(revision.program.as_deref().unwrap()).unwrap();
    assert_eq!(document.bodies.len(), 2);
    for id in ["body", "part"] {
        let solid = build_body(&document, id).unwrap();
        assert!((solid.volume_mm3() - 8000.).abs() < 1e-6);
    }
    let file = next
        .project
        .files
        .iter()
        .find(|file| Some(file.name.as_str()) == revision.source.as_deref())
        .unwrap();
    let path = root.path().join("split.step");
    std::fs::write(
        &path,
        forma_core::artifacts::read(&state, &source.id, file).unwrap(),
    )
    .unwrap();
    let imported = Solid::read_step(&path).unwrap();
    assert!((imported.volume_mm3() - 16000.).abs() < 1e-6);
    assert!((imported.bounds_mm()[0] - 90.).abs() < 1e-6);
    clean(&state).await;
}
