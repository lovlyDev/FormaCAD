//! Validate a modeling request before permissions or geometry execution.
use crate::{cad_document::CompiledDocument, core::Result};

pub struct PreparedProgram {
    pub typed: bool,
    pub compiled_legacy: Option<CompiledDocument>,
}

pub fn prepare(source: &str) -> Result<PreparedProgram> {
    if !source.trim_start().starts_with('{') {
        return Ok(PreparedProgram {
            typed: false,
            compiled_legacy: None,
        });
    }
    #[cfg(feature = "native-occt")]
    let value: serde_json::Value = serde_json::from_str(source)?;
    #[cfg(feature = "native-occt")]
    if value
        .get("schemaVersion")
        .and_then(serde_json::Value::as_u64)
        == Some(2)
    {
        let document: crate::cad_ir::Document = serde_json::from_value(value)?;
        document.validate().map_err(crate::cad_ir::app_error)?;
        return Ok(PreparedProgram {
            typed: true,
            compiled_legacy: None,
        });
    }
    Ok(PreparedProgram {
        typed: true,
        compiled_legacy: Some(crate::cad_ir::compile_json(source)?),
    })
}

#[cfg(test)]
mod legacy_tests {
    use super::*;
    #[test]
    fn legacy_typed_v1_retains_its_compiler_on_both_feature_configurations() {
        let source = r#"{"version":1,"features":[{"id":"profile","operation":{"type":"rectangle","width":40,"depth":20}},{"id":"pad","operation":{"type":"extrude","sketch":"profile","distance":10}}],"output":"pad"}"#;
        let prepared = prepare(source).unwrap();
        assert!(prepared.typed);
        assert!(prepared
            .compiled_legacy
            .as_ref()
            .unwrap()
            .source
            .contains("rect(40,20).extrude(10)"));
        let plain = prepare("result = base.faces('>Z').workplane().hole(4)").unwrap();
        assert!(!plain.typed);
        assert!(plain.compiled_legacy.is_none());
    }
}

#[cfg(all(test, feature = "native-occt"))]
mod tests {
    use super::*;

    #[test]
    fn native_multi_body_document_is_not_forced_through_one_body_adapter() {
        let source = serde_json::json!({
            "schemaVersion": 2,
            "revisionId": "revision_1",
            "parameters": [],
            "features": [
                {"id":"profile","name":"Profile","operation":{"type":"rectangle","width":{"kind":"literal","mm":10},"depth":{"kind":"literal","mm":20}}},
                {"id":"pad","name":"Pad","operation":{"type":"extrude","sketchId":"profile","distance":{"kind":"literal","mm":5}}}
            ],
            "bodies": [
                {"id":"housing","name":"Housing","sourceFeatureId":"pad"},
                {"id":"lid","name":"Lid","sourceFeatureId":"pad"}
            ]
        }).to_string();
        let prepared = prepare(&source).unwrap();
        assert!(prepared.typed);
        assert!(prepared.compiled_legacy.is_none());
    }
}
