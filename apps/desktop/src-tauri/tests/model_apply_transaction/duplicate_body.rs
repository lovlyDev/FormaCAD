use super::fixture::*;
use forma_core::{
    cad_ir::{Command, Document},
    modeling::{apply::apply_with_executable, commands::prepare},
};

#[tokio::test]
async fn duplicate_body_commits_exact_independent_geometry_and_rejects_id_collision() {
    let root = tempfile::tempdir().unwrap();
    let (state, baseline) = fixture(root.path()).await;
    let before = snapshot(&state, &baseline.id).await;
    let request = |new_body_id: &str| {
        prepare(
            &state,
            baseline.id.clone(),
            vec![Command::DuplicateBody {
                body_id: "body".into(),
                new_body_id: new_body_id.into(),
                name: "Copy".into(),
                offset_mm: [50., 0., 0.],
            }],
            "Duplicate body".into(),
            baseline.current_revision.clone().unwrap(),
        )
    };
    assert!(request("body").await.is_err());
    assert_eq!(snapshot(&state, &baseline.id).await, before);
    let next = apply_with_executable(&state, request("copy").await.unwrap(), executable())
        .await
        .unwrap();
    assert!(next.committed);
    assert_eq!(next.project.revisions.len(), baseline.revisions.len() + 1);
    let revision = next.project.revisions.last().unwrap();
    let document: Document = serde_json::from_str(revision.program.as_deref().unwrap()).unwrap();
    for id in ["body", "copy"] {
        let solid = forma_core::native::build_body(&document, id).unwrap();
        assert!((solid.volume_mm3() - 8000.).abs() < 1e-6);
    }
    assert_eq!(document.bodies.len(), 2);
    let file = next
        .project
        .files
        .iter()
        .find(|file| Some(file.name.as_str()) == revision.source.as_deref())
        .unwrap();
    let step = root.path().join("duplicate-round-trip.step");
    std::fs::write(
        &step,
        forma_core::artifacts::read(&state, &baseline.id, file).unwrap(),
    )
    .unwrap();
    let imported = forma_core::native::Solid::read_step(&step).unwrap();
    assert!((imported.volume_mm3() - 16000.).abs() < 1e-6);
    assert!((imported.bounds_mm()[0] - 90.).abs() < 1e-6);
    clean(&state).await;
}
