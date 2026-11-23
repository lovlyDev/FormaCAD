//! Synthetic intent tests: no worker, permission store, SQL or filesystem IO.
use super::*;
use crate::cad_ir::{Command, Document};
const OLD: &str = "11111111-1111-4111-8111-111111111111";
const NEXT: &str = "22222222-2222-4222-8222-222222222222";
const SOURCE: &str = "result = cq.Workplane('XY').box(40,20,10)";

fn project(source: &str) -> Project {
    serde_json::from_value(serde_json::json!({
        "schemaVersion":1,"id":"33333333-3333-4333-8333-333333333333",
        "name":"Prepared request fixture","units":"mm","agent":"codex","pinned":false,
        "createdAt":"2026-10-01T00:00:00Z","updatedAt":"2026-10-01T00:00:00Z",
        "currentRevision":OLD,"messages":[],"exports":[],"files":[],
        "revisions":[{"id":OLD,"parent":null,"createdAt":"2026-10-01T00:00:00Z",
            "prompt":"old","program":source,
            "parameters":{"kind":"box","width":40,"depth":20,"height":10,"thickness":2,"holeDiameter":0,"holes":0}}]
    })).unwrap()
}
fn request(project: &Project, source: &str) -> ApplyRequest {
    ApplyRequest::new(
        project.id.clone(),
        source.into(),
        "new prompt".into(),
        Some(OLD.into()),
    )
}
fn invalid(result: Result<PreparedRequest>, expected: &str) {
    match result {
        Err(AppError::Invalid(message)) => assert_eq!(message, expected),
        Err(other) => panic!("wrong error: {other}"),
        Ok(_) => panic!("invalid request accepted"),
    }
}
fn applied(result: Result<PreparedRequest>) -> PreparedApply {
    match result.unwrap() {
        PreparedRequest::Apply(value) => *value,
        PreparedRequest::NoOp => panic!("expected real work"),
    }
}
fn document() -> Document {
    serde_json::from_value(serde_json::json!({
        "schemaVersion":2,"revisionId":OLD,"parameters":[],
        "features":[
            {"id":"profile","name":"Profile","operation":{"type":"rectangle","width":{"kind":"literal","mm":40},"depth":{"kind":"literal","mm":20}}},
            {"id":"pad","name":"Pad","operation":{"type":"extrude","sketchId":"profile","distance":{"kind":"literal","mm":10}}}
        ],"bodies":[{"id":"body","name":"Body","sourceFeatureId":"pad"}]
    })).unwrap()
}

#[test]
fn limits_and_program_validation_keep_precedence_over_stale_revision() {
    let original = project(SOURCE);
    for (source, prompt) in [
        (" ".into(), "ok".into()),
        ("x".repeat(60001), "ok".into()),
        (SOURCE.into(), "x".repeat(16001)),
    ] {
        let mut value = request(&original, &source);
        value.prompt = prompt;
        value.expected_revision = None;
        invalid(
            prepare_with_job(&original, &value, || panic!("must not sample")),
            "Invalid modeling request",
        );
    }
    let mut malformed = request(&original, "{broken JSON");
    malformed.expected_revision = None;
    assert!(matches!(
        prepare_with_job(&original, &malformed, || panic!("must not sample")),
        Err(AppError::Json(_))
    ));
    let mut stale = request(&original, "different plain program");
    stale.expected_revision = None;
    invalid(
        prepare_with_job(&original, &stale, || panic!("must not sample")),
        "Project changed; regenerate the model from its current revision",
    );
}

#[test]
fn exact_limits_remain_accepted_and_job_is_sampled_after_validation() {
    let original = project(SOURCE);
    let mut value = request(&original, &"x".repeat(60000));
    value.prompt = "p".repeat(16000);
    let calls = std::cell::Cell::new(0);
    let prepared = applied(prepare_with_job(&original, &value, || {
        calls.set(calls.get() + 1);
        NEXT.into()
    }));
    assert_eq!(calls.get(), 1);
    assert_eq!(prepared.job.as_str(), NEXT);
    assert!(!prepared.program().typed);
    assert!(prepared.review_document().is_none());
    assert_eq!(prepared.saved_program, value.program);
}

#[test]
fn no_op_preserves_source_and_never_samples_or_uses_next_job() {
    let original = project(SOURCE);
    let before = serde_json::to_vec(&original).unwrap();
    let mut value = request(&original, &format!("  {SOURCE}  "));
    value.next_revision = Some("ignored-on-noop".into());
    assert!(matches!(
        prepare_with_job(&original, &value, || panic!("no-op must not sample")),
        Ok(PreparedRequest::NoOp)
    ));
    assert_eq!(serde_json::to_vec(&original).unwrap(), before);
}

#[test]
fn explicit_job_and_saved_program_share_existing_revision_binding() {
    let original = project(SOURCE);
    let mut value = request(&original, "  different plain program  ");
    value.next_revision = Some(NEXT.into());
    let prepared = applied(prepare_with_job(&original, &value, || {
        panic!("supplied job must be retained")
    }));
    assert_eq!(prepared.job.as_str(), NEXT);
    assert_eq!(
        prepared.saved_program,
        commit::program_for_revision(&value.program, false, NEXT).unwrap()
    );
}

#[cfg(feature = "native-worker-client")]
#[test]
fn review_plan_forces_work_owns_candidate_and_rebinds_only_saved_document() {
    let document = document();
    let source = serde_json::to_string(&document).unwrap();
    let original = project(&source);
    let before = serde_json::to_vec(&original).unwrap();
    let mut value = request(&original, &source);
    value.review_plan = Some(serde_json::from_value(serde_json::json!({
        "assumptions":[],"dimensions":[],"affectedBodyIds":["body"],"expectedChecks":[{"type":"validSolid"}]
    })).unwrap());
    let prepared = applied(prepare_with_job(&original, &value, || NEXT.into()));
    assert!(prepared.program().typed);
    assert!(prepared.program().compiled_legacy.is_none());
    assert_eq!(prepared.review_document(), Some(&document));
    let saved: Document = serde_json::from_str(&prepared.saved_program).unwrap();
    assert_eq!(saved.revision_id, NEXT);
    assert_eq!(saved.features, document.features);
    assert_eq!(serde_json::to_vec(&original).unwrap(), before);
    value
        .review_plan
        .as_mut()
        .unwrap()
        .affected_body_ids
        .push("missing".into());
    assert!(
        matches!(prepare_with_job(&original, &value, || panic!("invalid plan must not sample")), Err(AppError::Invalid(message)) if message.contains("REVIEW_PLAN_INVALID"))
    );
}

#[cfg(feature = "native-worker-client")]
#[test]
fn review_plan_can_reference_previous_body_and_rejects_unknown_body() {
    let current = document();
    let original = project(&serde_json::to_string(&current).unwrap());
    let mut candidate = current.clone();
    candidate.bodies[0].id = "replacement".into();
    let mut value = request(&original, &serde_json::to_string(&candidate).unwrap());
    value.review_plan = Some(
        serde_json::from_value(serde_json::json!({
            "assumptions":[],"dimensions":[],"affectedBodyIds":["body"],"expectedChecks":[]
        }))
        .unwrap(),
    );
    applied(prepare_with_job(&original, &value, || NEXT.into()));
    value.review_plan.as_mut().unwrap().affected_body_ids = vec!["unknown".into()];
    assert!(
        matches!(prepare_with_job(&original, &value, || panic!("invalid plan must not sample")), Err(AppError::Invalid(message)) if message.contains("REVIEW_PLAN_INVALID"))
    );
}

#[test]
fn snapshot_commands_use_the_single_engine_and_preserve_source() {
    let current = document();
    let original = project(&serde_json::to_string(&current).unwrap());
    let before = serde_json::to_vec(&original).unwrap();
    let commands = vec![Command::RenameFeature {
        feature_id: "profile".into(),
        name: "Changed profile".into(),
    }];
    let value = super::super::commands::prepare_for_snapshot(
        &original,
        commands.clone(),
        "command prompt".into(),
        OLD.into(),
    )
    .unwrap();
    let next = value.next_revision.as_deref().unwrap();
    assert_ne!(next, OLD);
    let expected = crate::cad_ir::apply_commands(&current, OLD, next, &commands).unwrap();
    assert_eq!(
        serde_json::from_str::<Document>(&value.program).unwrap(),
        expected
    );
    assert_eq!(value.project_id, original.id);
    assert_eq!(value.expected_revision.as_deref(), Some(OLD));
    assert_eq!(value.prompt, "command prompt");
    assert_eq!(serde_json::to_vec(&original).unwrap(), before);
}

#[test]
fn snapshot_commands_keep_count_stale_binding_and_no_change_errors() {
    let current = document();
    let original = project(&serde_json::to_string(&current).unwrap());
    let command = Command::RenameFeature {
        feature_id: "profile".into(),
        name: "Profile".into(),
    };
    let prepare = |project: &Project, commands: Vec<Command>, expected: &str| {
        super::super::commands::prepare_for_snapshot(
            project,
            commands,
            "prompt".into(),
            expected.into(),
        )
    };
    assert!(
        matches!(prepare(&original, vec![command.clone(); 257], "stale"), Err(AppError::Invalid(message)) if message == "Too many CAD commands")
    );
    assert!(
        matches!(prepare(&original, vec![command.clone()], "stale"), Err(AppError::Invalid(message)) if message == "Project changed; retry the CAD edit")
    );
    let mut without_program = original.clone();
    without_program.revisions[0].program = None;
    assert!(
        matches!(prepare(&without_program, vec![command.clone()], OLD), Err(AppError::Invalid(message)) if message == "Current revision has no CAD IR")
    );
    let mut mismatched = current.clone();
    mismatched.revision_id = NEXT.into();
    let mismatched = project(&serde_json::to_string(&mismatched).unwrap());
    assert!(
        matches!(prepare(&mismatched, vec![command.clone()], OLD), Err(AppError::Invalid(message)) if message == "CAD IR revision does not match project")
    );
    let expected =
        crate::cad_ir::apply_commands(&current, OLD, NEXT, std::slice::from_ref(&command))
            .unwrap_err();
    let actual = prepare(&original, vec![command], OLD).err().unwrap();
    assert_eq!(
        actual.to_string(),
        crate::cad_ir::app_error(expected).to_string()
    );
    let expected = crate::cad_ir::apply_commands(&current, OLD, NEXT, &[]).unwrap_err();
    assert_eq!(
        prepare(&original, vec![], OLD).err().unwrap().to_string(),
        crate::cad_ir::app_error(expected).to_string()
    );
}
