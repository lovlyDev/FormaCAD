//! Offset style is explicit and survives parameter changes and independent copies.
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProfileJoin {
    Intersection,
    Rounded,
}

#[cfg(test)]
mod tests {
    use crate::cad_ir::{commands::apply_commands, Command, Document};
    use serde_json::json;
    #[test]
    fn offset_profile_validates_dependencies_dimensions_and_independent_copy() {
        let value = json!({"schemaVersion":2,"revisionId":"base","parameters":[{"id":"offset","name":"Offset","valueMm":2}],"features":[
          {"id":"profile","name":"Profile","operation":{"type":"rectangle","width":{"kind":"literal","mm":40},"depth":{"kind":"literal","mm":20}}},
          {"id":"offsetProfile","name":"Offset","operation":{"type":"offsetProfile","sketchId":"profile","offset":{"kind":"parameter","parameterId":"offset"},"join":"intersection"}},
          {"id":"pad","name":"Pad","operation":{"type":"extrude","sketchId":"offsetProfile","distance":{"kind":"literal","mm":10}}}
        ],"bodies":[{"id":"body","name":"Body","sourceFeatureId":"pad"}]});
        let document: Document = serde_json::from_value(value.clone()).unwrap();
        document.validate().unwrap();
        assert_eq!(
            document.features[1].operation.dependencies(),
            vec!["profile"]
        );
        let commands:Vec<Command>=serde_json::from_value(json!([{"command":"duplicate_body","bodyId":"body","newBodyId":"copy","name":"Copy","offsetMm":[0,0,0]}])).unwrap();
        let copied = apply_commands(&document, "base", "copied", &commands).unwrap();
        let encoded = serde_json::to_value(copied).unwrap();
        assert_eq!(encoded["features"][4]["operation"]["sketchId"], "copy_f0");
        assert_eq!(
            encoded["features"][4]["operation"]["offset"]["parameterId"],
            "copy_p0"
        );
        for bad in [
            json!({"kind":"literal","mm":0}),
            json!({"kind":"literal","mm":10001}),
            json!({"kind":"parameter","parameterId":"missing"}),
        ] {
            let mut invalid = value.clone();
            invalid["features"][1]["operation"]["offset"] = bad;
            assert!(serde_json::from_value::<Document>(invalid)
                .unwrap()
                .validate()
                .is_err());
        }
        let mut invalid = value;
        invalid["features"][1]["operation"]["sketchId"] = "pad".into();
        assert!(serde_json::from_value::<Document>(invalid)
            .unwrap()
            .validate()
            .is_err());
    }
}
