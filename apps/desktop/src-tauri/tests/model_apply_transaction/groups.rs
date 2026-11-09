use super::fixture::*;
use forma_core::{
    cad_ir::{BodyGroup, Command, Document},
    modeling::{apply::apply_with_executable, commands::prepare},
    native::Solid,
};

#[tokio::test]
async fn group_edits_are_revisioned_and_keep_exact_geometry() {
    let root = tempfile::tempdir().unwrap();
    let (state, baseline) = fixture(root.path()).await;
    let before = snapshot(&state, &baseline.id).await;
    let request = |members| {
        prepare(
            &state,
            baseline.id.clone(),
            vec![Command::GroupCreate {
                group: BodyGroup {
                    id: "group".into(),
                    name: "Group".into(),
                    body_ids: members,
                },
            }],
            "Create group".into(),
            baseline.current_revision.clone().unwrap(),
        )
    };
    assert!(request(vec!["missing".into()]).await.is_err());
    assert_eq!(snapshot(&state, &baseline.id).await, before);
    let next = apply_with_executable(
        &state,
        request(vec!["body".into()]).await.unwrap(),
        executable(),
    )
    .await
    .unwrap();
    assert!(next.committed);
    let revision = next.project.revisions.last().unwrap();
    let document: Document = serde_json::from_str(revision.program.as_deref().unwrap()).unwrap();
    assert_eq!(document.groups[0].body_ids, ["body"]);
    let ungrouped = forma_core::cad_ir::apply_commands(
        &document,
        &document.revision_id,
        "ungrouped",
        &[Command::GroupRemove {
            group_id: "group".into(),
            body_ids: None,
        }],
    )
    .unwrap();
    assert!(ungrouped.groups.is_empty());
    assert_eq!(ungrouped.bodies, document.bodies);
    let file = next
        .project
        .files
        .iter()
        .find(|file| Some(file.name.as_str()) == revision.source.as_deref())
        .unwrap();
    let path = root.path().join("group.step");
    std::fs::write(
        &path,
        forma_core::artifacts::read(&state, &baseline.id, file).unwrap(),
    )
    .unwrap();
    let imported = Solid::read_step(&path).unwrap();
    assert!((imported.volume_mm3() - 8000.).abs() < 1e-6);
    assert!((imported.bounds_mm()[0] - 40.).abs() < 1e-6);
    clean(&state).await;
}
