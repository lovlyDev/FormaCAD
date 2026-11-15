//! Contract tests for endpoint radius parameters, provenance and body copies.
#[cfg(test)]
mod tests {
    use crate::cad_ir::{commands::apply_commands, Command, Document};
    use serde_json::json;
    #[test]
    fn endpoint_measurements_require_provenance_and_two_finite_distinct_points() {
        use crate::model_measurement::schema::{MeasurementQuery, MeasurementValue};
        let value = json!({"kind":"edgeEndpoints","reference":{"schemaVersion":1,"kind":"edge","ownerFeatureId":"pad","role":"box-edge:z:xmax:ymax","occurrencePath":[]}});
        let query: MeasurementQuery = serde_json::from_value(value.clone()).unwrap();
        assert!(query.valid());
        assert!(MeasurementValue::EdgeEndpoints {
            start_mm: [20., 10., 0.],
            end_mm: [20., 10., 10.]
        }
        .valid_for(&query));
        assert!(!MeasurementValue::EdgeEndpoints {
            start_mm: [0.; 3],
            end_mm: [0.; 3]
        }
        .valid_for(&query));
        assert!(!MeasurementValue::EdgeEndpoints {
            start_mm: [f64::NAN, 0., 0.],
            end_mm: [0., 0., 10.]
        }
        .valid_for(&query));
        for extra in ["startMm", "edgeOrdinal", "sourcePath"] {
            let mut forged = value.clone();
            forged[extra] = json!(1);
            assert!(serde_json::from_value::<MeasurementQuery>(forged).is_err());
        }
    }
    #[test]
    fn variable_fillet_preserves_endpoints_dependencies_and_independent_parameters() {
        let value = json!({"schemaVersion":2,"revisionId":"base","parameters":[{"id":"r1","name":"Start","valueMm":1},{"id":"r2","name":"End","valueMm":3}],"features":[
          {"id":"profile","name":"Profile","operation":{"type":"rectangle","width":{"kind":"literal","mm":40},"depth":{"kind":"literal","mm":20}}},
          {"id":"pad","name":"Pad","operation":{"type":"extrude","sketchId":"profile","distance":{"kind":"literal","mm":10}}},
          {"id":"fillet","name":"Fillet","operation":{"type":"variableFillet","bodyFeatureId":"pad","reference":{"schemaVersion":1,"kind":"edge","ownerFeatureId":"pad","role":"box-edge:z:xmax:ymax","occurrencePath":[]},"radiusStart":{"kind":"parameter","parameterId":"r1"},"radiusEnd":{"kind":"parameter","parameterId":"r2"}}}
        ],"bodies":[{"id":"body","name":"Body","sourceFeatureId":"fillet"}]});
        let document: Document = serde_json::from_value(value.clone()).unwrap();
        document.validate().unwrap();
        assert_eq!(
            document.features[2].operation.dependencies(),
            vec!["pad", "pad"]
        );
        assert_eq!(
            document.features[2].operation.suppression_source(),
            Some("pad")
        );
        let commands:Vec<Command>=serde_json::from_value(json!([{"command":"duplicate_body","bodyId":"body","newBodyId":"copy","name":"Copy","offsetMm":[0,0,0]}])).unwrap();
        let copy = apply_commands(&document, "base", "copied", &commands).unwrap();
        let encoded = serde_json::to_value(copy).unwrap();
        assert_eq!(
            encoded["features"][5]["operation"]["reference"]["ownerFeatureId"],
            "copy_f1"
        );
        for (field, expected) in [("radiusStart", 1.), ("radiusEnd", 3.)] {
            let parameter_id = encoded["features"][5]["operation"][field]["parameterId"]
                .as_str()
                .unwrap();
            assert!(parameter_id.starts_with("copy_p"));
            let parameter = encoded["parameters"]
                .as_array()
                .unwrap()
                .iter()
                .find(|p| p["id"] == parameter_id)
                .unwrap();
            assert_eq!(parameter["valueMm"].as_f64().unwrap(), expected);
        }
        for field in ["radiusStart", "radiusEnd"] {
            for bad in [
                json!({"kind":"literal","mm":0}),
                json!({"kind":"literal","mm":-1}),
                json!({"kind":"literal","mm":10001}),
                json!({"kind":"parameter","parameterId":"missing"}),
            ] {
                let mut invalid = value.clone();
                invalid["features"][2]["operation"][field] = bad;
                assert!(serde_json::from_value::<Document>(invalid)
                    .unwrap()
                    .validate()
                    .is_err());
            }
        }
    }
}
