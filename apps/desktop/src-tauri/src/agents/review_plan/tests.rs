use super::*;
fn document() -> Document {
    serde_json::from_str(include_str!(
        "../../../../../../docs/fixtures/parameterized-disc.cad.json"
    ))
    .unwrap()
}
fn plan() -> ReviewPlan {
    serde_json::from_value(serde_json::json!({
        "assumptions":["All dimensions are millimetres"],
        "dimensions":[{"name":"Thickness","valueMm":8,"parameterId":"thickness"}],
        "affectedBodyIds":["disc_body"],
        "expectedChecks":[{"type":"validSolid"},{"type":"bodyCount","count":1},{"type":"bounds","sizeMm":[40,20,10],"toleranceMm":0.01},{"type":"volume","valueMm3":8000,"toleranceMm3":0.01}]
    })).unwrap()
}
#[test]
fn checks_bounded_types_parameter_values_and_body_ids() {
    let doc = document();
    let valid = plan();
    valid.validate(&doc, None).unwrap();
    for variant in 0..6 {
        let mut bad = valid.clone();
        match variant {
            0 => bad.dimensions[0].value_mm = 9.0,
            1 => bad.dimensions[0].parameter_id = Some("missing".into()),
            2 => bad.affected_body_ids.push("missing".into()),
            3 => bad.affected_body_ids.push("disc_body".into()),
            4 => bad.assumptions = vec!["x".repeat(401)],
            _ => bad.expected_checks.push(ExpectedCheck::Bounds {
                size_mm: [40.0, 20.0, 10.0],
                tolerance_mm: -1.0,
            }),
        }
        assert!(bad.validate(&doc, None).is_err());
    }
    let mut moved = valid.clone();
    moved.affected_body_ids = vec!["old_body".into()];
    let mut old = doc.clone();
    old.bodies[0].id = "old_body".into();
    moved.validate(&doc, Some(&old)).unwrap();
    assert!(moved.validate(&doc, None).is_err());
}
#[test]
fn strict_plan_unknown_fields_and_legacy_absence_are_explicit() {
    let doc = document();
    assert!(parse(None, None, None).unwrap().is_none());
    assert!(parse(Some(&serde_json::Value::Null), None, None)
        .unwrap()
        .is_none());
    let mut raw = serde_json::to_value(plan()).unwrap();
    raw["extraPermission"] = "allow".into();
    assert!(parse(Some(&raw), Some(&doc), None).is_err());
    assert!(parse(Some(&serde_json::to_value(plan()).unwrap()), None, None).is_err());
}
#[test]
fn expectations_use_exact_worker_metrics_and_document_body_count() {
    let doc = document();
    let plan = plan();
    let result = serde_json::json!({"status":"completed","volumeMm3":8000,"boundsMm":[40,20,10]});
    check_metrics(&plan, &doc, &result).unwrap();
    for field in ["status", "volumeMm3", "boundsMm"] {
        let mut invalid = result.clone();
        invalid[field] = serde_json::Value::Null;
        assert!(check_metrics(&plan, &doc, &invalid).is_err());
    }
    let mut invalid = result.clone();
    invalid["volumeMm3"] = 7999.into();
    assert!(check_metrics(&plan, &doc, &invalid)
        .unwrap_err()
        .to_string()
        .contains("EXPECTED_CHECK_FAILED"));
    let mut two = doc.clone();
    two.bodies.push(doc.bodies[0].clone());
    assert!(check_metrics(&plan, &two, &result).is_err());
}
#[test]
#[cfg(feature = "native-occt")]
fn native_and_command_proposals_carry_validated_plan_without_changing_base() {
    let doc = document();
    let original = serde_json::to_string(&doc).unwrap();
    let response = serde_json::json!({"message":"Plan","cad":doc,"reviewPlan":plan()});
    let result =
        crate::agents::protocol::parse_native_output("custom", &response.to_string()).unwrap();
    assert!(result.review_plan.is_some());
    let mut changed = plan();
    changed.dimensions[0].value_mm = 12.0;
    let response = serde_json::json!({"message":"Plan","expectedRevision":doc.revision_id,"commands":[{"command":"set_parameter","parameterId":"thickness","valueMm":12}],"reviewPlan":changed});
    let staged =
        crate::agents::protocol::parse_native_edit_output("custom", &response.to_string(), &doc)
            .unwrap();
    assert_eq!(staged.review_plan.unwrap().dimensions[0].value_mm, 12.0);
    assert_eq!(serde_json::to_string(&doc).unwrap(), original);
}
