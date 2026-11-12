//! Explicit contiguous ranges; generators have an empty material base.
use super::{Feature, Operation};
use std::collections::HashSet;
pub(crate) const MAX_RANGE: usize = 64;
pub(crate) enum Input<'a> {
    Empty,
    Body(&'a str),
    Unsupported,
}
pub(crate) fn input(operation: &Operation) -> Input<'_> {
    match operation {
        Operation::Boolean {
            left_feature_id, ..
        } => Input::Body(left_feature_id),
        Operation::Hole {
            body_feature_id, ..
        }
        | Operation::Fillet {
            body_feature_id, ..
        }
        | Operation::FilletEdge {
            body_feature_id, ..
        }
        | Operation::FilletReferencedEdge {
            body_feature_id, ..
        }
        | Operation::Chamfer {
            body_feature_id, ..
        }
        | Operation::ChamferReferencedEdge {
            body_feature_id, ..
        }
        | Operation::ExtrudeFace {
            body_feature_id, ..
        }
        | Operation::Shell {
            body_feature_id, ..
        }
        | Operation::SplitPlane {
            body_feature_id, ..
        }
        | Operation::PatternLinear {
            body_feature_id, ..
        }
        | Operation::PatternCircular {
            body_feature_id, ..
        }
        | Operation::MirrorFeatures {
            body_feature_id, ..
        } => Input::Body(body_feature_id),
        Operation::Extrude { .. }
        | Operation::Revolve { .. }
        | Operation::Sphere { .. }
        | Operation::Cone { .. }
        | Operation::Torus { .. }
        | Operation::ImportStep { .. }
        | Operation::ThickenFace { .. }
        | Operation::ExtractSolid { .. } => Input::Empty,
        _ => Input::Unsupported,
    }
}
pub(crate) fn valid_range(
    features: &[Feature],
    base: Option<&str>,
    source: &str,
    ids: &[String],
) -> bool {
    if ids.is_empty() || ids.len() > MAX_RANGE || ids.last().map(String::as_str) != Some(source) {
        return false;
    }
    let mut seen = HashSet::new();
    if let Some(id) = base {
        seen.insert(id);
    }
    let mut previous = base;
    for (index, id) in ids.iter().enumerate() {
        if !seen.insert(id.as_str()) {
            return false;
        }
        let Some(feature) = features.iter().find(|feature| feature.id == *id) else {
            return false;
        };
        match input(&feature.operation) {
            Input::Empty if index == 0 && previous.is_none() => {}
            Input::Body(parent) if Some(parent) == previous => {}
            _ => return false,
        }
        previous = Some(id);
    }
    true
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::cad_ir::{commands::apply_commands, Command, Document};
    use serde_json::json;
    fn document() -> Document {
        serde_json::from_value(json!({"schemaVersion":2,"revisionId":"baseline","parameters":[],"features":[
          {"id":"seed","name":"Seed","operation":{"type":"sphere","radius":{"kind":"literal","mm":10}}},
          {"id":"round","name":"Round","operation":{"type":"fillet","bodyFeatureId":"seed","radius":{"kind":"literal","mm":1}}},
          {"id":"bevel","name":"Bevel","operation":{"type":"chamfer","bodyFeatureId":"round","distance":{"kind":"literal","mm":1}}}
        ],"bodies":[{"id":"body","name":"Body","sourceFeatureId":"bevel"}]})).unwrap()
    }
    #[test]
    fn range_is_ordered_bounded_and_ends_at_the_active_source() {
        let doc = document();
        let ids = |names: &[&str]| {
            names
                .iter()
                .map(|name| name.to_string())
                .collect::<Vec<_>>()
        };
        assert!(valid_range(
            &doc.features,
            Some("seed"),
            "bevel",
            &ids(&["round", "bevel"])
        ));
        assert!(valid_range(
            &doc.features,
            None,
            "bevel",
            &ids(&["seed", "round", "bevel"])
        ));
        assert!(!valid_range(
            &doc.features,
            Some("seed"),
            "bevel",
            &ids(&["bevel"])
        ));
        assert!(!valid_range(
            &doc.features,
            None,
            "bevel",
            &ids(&["round", "bevel"])
        ));
        assert!(!valid_range(
            &doc.features,
            Some("seed"),
            "round",
            &ids(&["bevel", "round"])
        ));
        assert!(!valid_range(
            &doc.features,
            Some("seed"),
            "bevel",
            &ids(&["round", "round", "bevel"])
        ));
    }
    #[test]
    fn independent_copy_remaps_both_range_and_optional_base() {
        for base in [Some("seed"), None] {
            let mut doc = document();
            let ids: Vec<String> = if base.is_some() {
                vec!["round".into(), "bevel".into()]
            } else {
                vec!["seed".into(), "round".into(), "bevel".into()]
            };
            doc.features.push(serde_json::from_value(json!({"id":"mirrored","name":"Mirror","operation":{"type":"mirrorFeatures","bodyFeatureId":"bevel","baseFeatureId":base,"featureIds":ids,"planeOriginMm":[0,0,0],"planeNormal":[1,0,0]}})).unwrap());
            doc.bodies[0].source_feature_id = "mirrored".into();
            doc.validate().unwrap();
            let copied = apply_commands(
                &doc,
                "baseline",
                "next",
                &[Command::DuplicateBody {
                    body_id: "body".into(),
                    new_body_id: "copy".into(),
                    name: "Copy".into(),
                    offset_mm: [30., 0., 0.],
                }],
            )
            .unwrap();
            assert_eq!(&copied.features[..doc.features.len()], &doc.features);
            let operation = copied
                .features
                .iter()
                .skip(doc.features.len())
                .find_map(|feature| match &feature.operation {
                    Operation::MirrorFeatures {
                        body_feature_id,
                        base_feature_id,
                        feature_ids,
                        ..
                    } => Some((body_feature_id, base_feature_id, feature_ids)),
                    _ => None,
                })
                .unwrap();
            assert!(operation.0.starts_with("copy_f"));
            assert_eq!(operation.1.is_some(), base.is_some());
            assert!(operation
                .1
                .as_ref()
                .is_none_or(|id| id.starts_with("copy_f")));
            assert!(operation.2.iter().all(|id| id.starts_with("copy_f")));
        }
    }
}
