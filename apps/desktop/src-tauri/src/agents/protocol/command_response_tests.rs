use super::*;
fn fixture() -> crate::cad_ir::Document {
    serde_json::from_str(include_str!(
        "../../../../../../docs/fixtures/linked-profiles.cad.json"
    ))
    .unwrap()
}
fn proposal() -> serde_json::Value {
    serde_json::json!({"message":"Change width and unlink one axis","expectedRevision":"draft","commands":[{"command":"set_parameter","parameterId":"width","valueMm":80},{"command":"remove_sketch_binding","featureId":"base_sketch","bindingId":"a_x"}]})
}
#[test]
fn native_command_proposals_stage_on_the_captured_revision_for_every_provider() {
    let current = fixture();
    let before = current.clone();
    let proposal = proposal();
    for provider in ["custom", "codex", "claude"] {
        let output=match provider {"codex"=>serde_json::json!({"type":"item.completed","item":{"type":"agent_message","text":proposal.to_string()}}).to_string(),"claude"=>serde_json::json!({"type":"result","result":proposal.to_string()}).to_string(),_=>proposal.to_string()};
        let plan = parse_native_edit_output(provider, &output, &current).unwrap();
        let staged: crate::cad_ir::Document = serde_json::from_str(&plan.program.unwrap()).unwrap();
        assert_eq!(staged.parameters[0].value_mm, 80.0);
        assert_ne!(staged.revision_id, current.revision_id);
        assert_eq!(current, before);
        let crate::cad_ir::Operation::Sketch2d {
            points, bindings, ..
        } = &staged.features[0].operation
        else {
            panic!()
        };
        assert_eq!(points[0].x_mm, -40.0);
        assert_eq!(bindings.len(), 7);
    }
}
#[test]
fn mixed_stale_unknown_empty_and_oversized_proposals_are_rejected() {
    let current = fixture();
    for variant in 0..7 {
        let mut proposal = proposal();
        match variant {
            0 => proposal["expectedRevision"] = "stale".into(),
            1 => proposal["cad"] = serde_json::json!({}),
            2 => proposal["commands"][1]["command"] = "delete_everything".into(),
            3 => proposal["commands"] = serde_json::json!([]),
            4 => proposal["commands"][1]["bindingId"] = "missing".into(),
            5 => {
                let edit = proposal["commands"][0].clone();
                proposal["commands"] = serde_json::Value::Array(vec![edit; 257]);
            }
            _ => proposal["unexpectedFlag"] = true.into(),
        };
        assert!(
            parse_native_edit_output("custom", &proposal.to_string(), &current).is_err(),
            "{proposal}"
        );
    }
    assert!(parse_native_output("custom", &proposal().to_string()).is_err());
    assert!(parse_output("custom", &proposal().to_string()).is_err());
}
#[test]
fn complete_documents_keep_bindings_and_protocol_describes_current_schema() {
    let current = fixture();
    let response = serde_json::json!({"message":"Linked profiles","cad":current});
    let parsed = parse_native_output("custom", &response.to_string()).unwrap();
    let decoded: crate::cad_ir::Document = serde_json::from_str(&parsed.program.unwrap()).unwrap();
    assert_eq!(decoded, current);
    let protocol = include_str!("../../../scripts/native_protocol.txt");
    for field in [
        "bindings",
        "profiles",
        "construction",
        "add_sketch_binding",
        "set_sketch_binding",
        "remove_sketch_binding",
        "expectedRevision",
    ] {
        assert!(protocol.contains(field), "{field}");
    }
}
