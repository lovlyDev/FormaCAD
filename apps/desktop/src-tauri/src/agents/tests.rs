use super::protocol::public_event;
use super::*;

#[test]
fn typed_document_round_trips_through_agent_protocol() {
    let cad: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../docs/fixtures/plate-hole.cad.json"
    ))
    .unwrap();
    let response = serde_json::json!({"message":"Plate", "cad":cad});
    let event = serde_json::json!({"type":"item.completed","item":{"type":"agent_message","text":response.to_string()}});
    let parsed = parse_output("codex", &event.to_string()).unwrap();
    let stored: serde_json::Value = serde_json::from_str(&parsed.program.unwrap()).unwrap();
    assert_eq!(stored, cad);
    let response = serde_json::json!({"message":"Plate", "cad":cad, "program":"result = base"});
    let event = serde_json::json!({"type":"item.completed","item":{"type":"agent_message","text":response.to_string()}});
    assert!(parse_output("codex", &event.to_string()).is_err());
}
#[cfg(feature = "native-occt")]
#[test]
fn native_agent_accepts_ir_v2_and_rejects_python_fallback() {
    let cad = serde_json::json!({
        "schemaVersion": 2,
        "revisionId": "draft",
        "parameters": [],
        "features": [{
            "id": "ball",
            "name": "Ball",
            "operation": {"type":"sphere","radius":{"kind":"literal","mm":12.0}}
        }],
        "bodies": [{"id":"body","name":"Ball","sourceFeatureId":"ball"}]
    });
    let response = serde_json::json!({"message":"Шар", "cad":cad});
    let event = serde_json::json!({"type":"item.completed","item":{"type":"agent_message","text":response.to_string()}});
    assert!(protocol::parse_native_output("codex", &event.to_string())
        .unwrap()
        .program
        .unwrap()
        .contains("sphere"));
    let response =
        serde_json::json!({"message":"Шар", "program":"result = cq.Workplane('XY').sphere(12)"});
    let event = serde_json::json!({"type":"item.completed","item":{"type":"agent_message","text":response.to_string()}});
    assert!(protocol::parse_native_output("codex", &event.to_string()).is_err());
    let direct = serde_json::json!({"message":"Done", "cad":cad}).to_string();
    assert!(protocol::parse_native_output("custom", &direct)
        .unwrap()
        .program
        .is_some());
}
#[test]
fn blank_result_is_not_a_success() {
    let p = serde_json::json!({"kind":"blank","width":120,"depth":65,"height":60,"thickness":5,"holeDiameter":8,"holes":4});
    let event = serde_json::json!({"type":"item.completed","item":{"type":"agent_message","text":p.to_string()}});
    assert!(parse_output("codex", &event.to_string()).is_err());
}
#[test]
fn public_messages_and_status_are_forwarded() {
    assert_eq!(
        public_event(r#"{"type":"thread.started"}"#).unwrap().0,
        "connected"
    );
    assert!(public_event(
        r#"{"type":"item.completed","item":{"type":"reasoning","text":"private"}}"#
    )
    .is_none());
    let response = serde_json::json!({"message":"Ready","program":null});
    let line=serde_json::json!({"type":"item.completed","item":{"type":"agent_message","text":response.to_string()}}).to_string();
    assert_eq!(public_event(&line).unwrap().1, "Ready");
    assert!(parse_output("codex", &line).unwrap().program.is_none());
}
#[test]
fn malformed_geometry_is_rejected() {
    assert!(parse_output(
        "codex",
        "{\"type\":\"item.completed\",\"item\":{\"type\":\"agent_message\",\"text\":\"{}\"}}"
    )
    .is_err());
    assert!(parse_output("codex", "not json").is_err());
}
#[test]
fn valid_structured_event_is_read() {
    let p =
        serde_json::json!({"message":"Шар", "program":"result = cq.Workplane(\"XY\").sphere(25)"});
    let event = serde_json::json!({"type":"item.completed","item":{"type":"agent_message","text":p.to_string()}});
    assert!(parse_output("codex", &event.to_string())
        .unwrap()
        .program
        .unwrap()
        .contains("sphere"));
}
