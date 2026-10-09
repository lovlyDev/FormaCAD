//! Deterministic CAD IR v2 execution against OCCT. A caller owns the transaction
//! and only persists the returned solid after geometry and references validate.
use super::{NativeError, Solid};
use crate::cad_ir::{BooleanMode, Dimension, Document, Operation};
use std::collections::HashMap;

enum FeatureValue {
    Sketch(crate::cad_ir::sketch::SolvedSketch),
    Rectangle(f64, f64),
    Circle(f64),
    Solid(Solid),
    Alias(String),
}

fn resolved_key(features: &HashMap<&str, FeatureValue>, id: &str) -> Result<String, NativeError> {
    let mut current = id.to_owned();
    for _ in 0..=features.len() {
        match features.get(current.as_str()) {
            Some(FeatureValue::Alias(source)) => current = source.clone(),
            Some(_) => return Ok(current),
            None => return Err(ir_error("BROKEN_REFERENCE", current)),
        }
    }
    Err(ir_error("BROKEN_REFERENCE", id))
}

fn feature_value<'a>(
    features: &'a HashMap<&str, FeatureValue>,
    id: &str,
) -> Result<&'a FeatureValue, NativeError> {
    let key = resolved_key(features, id)?;
    features
        .get(key.as_str())
        .ok_or_else(|| ir_error("BROKEN_REFERENCE", id))
}

fn ir_error(code: &str, detail: impl Into<String>) -> NativeError {
    NativeError {
        code: code.into(),
        detail: detail.into(),
    }
}

fn dimension(value: &Dimension, parameters: &HashMap<&str, f64>) -> Result<f64, NativeError> {
    match value {
        Dimension::Literal { mm } => Ok(*mm),
        Dimension::Parameter { parameter_id } => parameters
            .get(parameter_id.as_str())
            .copied()
            .ok_or_else(|| ir_error("BROKEN_REFERENCE", parameter_id)),
    }
}

/// Build a named body from its ordered features. No project files are written.
pub fn build_body(document: &Document, body_id: &str) -> Result<Solid, NativeError> {
    build_body_with_assets(document, body_id, None)
}

pub fn build_body_with_assets(
    document: &Document,
    body_id: &str,
    asset_directory: Option<&std::path::Path>,
) -> Result<Solid, NativeError> {
    document.validate().map_err(|error| {
        ir_error(
            "CAD_IR_INVALID",
            format!("{:?}: {:?}", error.code, error.target_id),
        )
    })?;
    let body = document
        .bodies
        .iter()
        .find(|body| body.id == body_id)
        .ok_or_else(|| ir_error("MISSING_BODY", body_id))?;
    let parameters: HashMap<&str, f64> = document
        .parameters
        .iter()
        .map(|parameter| (parameter.id.as_str(), parameter.value_mm))
        .collect();
    let mut features = HashMap::<&str, FeatureValue>::with_capacity(document.features.len());
    let needed = crate::cad_ir::required_features(document, &body.source_feature_id);
    for feature in &document.features {
        if !needed.contains(feature.id.as_str()) {
            continue;
        }
        if feature.suppressed {
            let source = feature
                .operation
                .suppression_source()
                .ok_or_else(|| ir_error("UNSUPPORTED_SUPPRESSION", &feature.id))?;
            features.insert(feature.id.as_str(), FeatureValue::Alias(source.to_owned()));
            continue;
        }
        let get = |id: &str| feature_value(&features, id);
        let value = match &feature.operation {
            Operation::ImportStep { asset_id, sha256 } => {
                let directory = asset_directory
                    .ok_or_else(|| ir_error("ASSET_MISSING", "STEP asset resolver is required"))?;
                FeatureValue::Solid(super::assets::read_step(directory, asset_id, sha256)?)
            }
            Operation::Sketch2d {
                plane,
                origin_mm,
                points,
                lines,
                constraints,
                bindings,
            } => FeatureValue::Sketch(
                crate::cad_ir::sketch::solve_bound(
                    *plane,
                    *origin_mm,
                    points,
                    lines,
                    constraints,
                    bindings,
                    &parameters,
                    &feature.id,
                )
                .map_err(|error| match error.code {
                    crate::cad_ir::ErrorCode::SketchConstraintConflict => {
                        ir_error("SKETCH_CONSTRAINT_CONFLICT", "Sketch constraints conflict")
                    }
                    _ => ir_error("INVALID_SKETCH", "Sketch profile is invalid"),
                })?,
            ),
            Operation::Rectangle { width, depth } => FeatureValue::Rectangle(
                dimension(width, &parameters)?,
                dimension(depth, &parameters)?,
            ),
            Operation::Circle { radius } => FeatureValue::Circle(dimension(radius, &parameters)?),
            Operation::Sphere { radius } => {
                FeatureValue::Solid(Solid::sphere(dimension(radius, &parameters)?)?)
            }
            Operation::Cone {
                bottom_radius,
                top_radius,
                height,
            } => FeatureValue::Solid(Solid::cone(
                dimension(bottom_radius, &parameters)?,
                dimension(top_radius, &parameters)?,
                dimension(height, &parameters)?,
            )?),
            Operation::Extrude {
                sketch_id,
                distance,
            } => {
                let height = dimension(distance, &parameters)?;
                let solid = match get(sketch_id)? {
                    FeatureValue::Rectangle(width, depth) => {
                        if height > 0.0 {
                            Solid::referenced_box(*width, *depth, height, &feature.id)?
                        } else {
                            Solid::box_solid(*width, *depth, height.abs())?
                        }
                    }
                    FeatureValue::Circle(radius) => {
                        if height > 0.0 {
                            Solid::referenced_cylinder(*radius, height, &feature.id)?
                        } else {
                            Solid::cylinder(*radius, height.abs())?
                        }
                    }
                    FeatureValue::Sketch(sketch) => Solid::polygon_prism(sketch, height)?,
                    FeatureValue::Solid(_) | FeatureValue::Alias(_) => {
                        return Err(ir_error("REFERENCE_TYPE_MISMATCH", sketch_id));
                    }
                };
                FeatureValue::Solid(if height < 0.0 {
                    solid.translated(0.0, 0.0, height)?
                } else {
                    solid
                })
            }
            Operation::Revolve {
                sketch_id,
                axis_origin_mm,
                axis_direction,
                angle_deg,
            } => match get(sketch_id)? {
                FeatureValue::Sketch(sketch) => FeatureValue::Solid(Solid::revolved_profile(
                    sketch,
                    *axis_origin_mm,
                    *axis_direction,
                    *angle_deg,
                )?),
                _ => return Err(ir_error("REFERENCE_TYPE_MISMATCH", sketch_id)),
            },
            Operation::Hole {
                body_feature_id,
                center_mm,
                start_z_mm,
                radius,
                depth,
            } => match get(body_feature_id)? {
                FeatureValue::Solid(source) => FeatureValue::Solid(super::hole::cut_hole(
                    source,
                    *center_mm,
                    *start_z_mm,
                    dimension(radius, &parameters)?,
                    dimension(depth, &parameters)?,
                )?),
                _ => return Err(ir_error("REFERENCE_TYPE_MISMATCH", body_feature_id)),
            },
            Operation::Fillet {
                body_feature_id,
                radius,
            } => match get(body_feature_id)? {
                FeatureValue::Solid(source) => {
                    FeatureValue::Solid(source.fillet_all_edges(dimension(radius, &parameters)?)?)
                }
                _ => return Err(ir_error("REFERENCE_TYPE_MISMATCH", body_feature_id)),
            },
            Operation::FilletEdge {
                body_feature_id,
                edge_key,
                radius,
            } => match get(body_feature_id)? {
                FeatureValue::Solid(source) => FeatureValue::Solid(
                    source.fillet_edge(edge_key, dimension(radius, &parameters)?)?,
                ),
                _ => return Err(ir_error("REFERENCE_TYPE_MISMATCH", body_feature_id)),
            },
            Operation::Chamfer {
                body_feature_id,
                distance,
            } => match get(body_feature_id)? {
                FeatureValue::Solid(source) => FeatureValue::Solid(
                    source.chamfer_all_edges(dimension(distance, &parameters)?)?,
                ),
                _ => return Err(ir_error("REFERENCE_TYPE_MISMATCH", body_feature_id)),
            },
            Operation::FilletReferencedEdge {
                body_feature_id,
                reference,
                radius,
            } => match get(body_feature_id)? {
                FeatureValue::Solid(source) => FeatureValue::Solid(
                    source.fillet_referenced_edge(reference, dimension(radius, &parameters)?)?,
                ),
                _ => return Err(ir_error("REFERENCE_TYPE_MISMATCH", body_feature_id)),
            },
            Operation::Translate {
                body_feature_id,
                offset_mm,
            } => match get(body_feature_id)? {
                FeatureValue::Solid(source) => FeatureValue::Solid(
                    source
                        .translated(offset_mm[0], offset_mm[1], offset_mm[2])?
                        .with_topology_occurrence(&feature.id)?,
                ),
                _ => return Err(ir_error("REFERENCE_TYPE_MISMATCH", body_feature_id)),
            },
            Operation::Rotate {
                body_feature_id,
                axis_origin_mm,
                axis_direction,
                angle_deg,
            } => match get(body_feature_id)? {
                FeatureValue::Solid(source) => FeatureValue::Solid(
                    source
                        .rotated(*axis_origin_mm, *axis_direction, *angle_deg)?
                        .with_topology_occurrence(&feature.id)?,
                ),
                _ => return Err(ir_error("REFERENCE_TYPE_MISMATCH", body_feature_id)),
            },
            Operation::Mirror {
                body_feature_id,
                plane_origin_mm,
                plane_normal,
            } => match get(body_feature_id)? {
                FeatureValue::Solid(source) => FeatureValue::Solid(
                    source
                        .mirrored(*plane_origin_mm, *plane_normal)?
                        .with_topology_occurrence(&feature.id)?,
                ),
                _ => return Err(ir_error("REFERENCE_TYPE_MISMATCH", body_feature_id)),
            },
            Operation::Boolean {
                left_feature_id,
                right_feature_id,
                mode,
            } => {
                let left = match get(left_feature_id)? {
                    FeatureValue::Solid(solid) => solid,
                    _ => return Err(ir_error("REFERENCE_TYPE_MISMATCH", left_feature_id)),
                };
                let right = match get(right_feature_id)? {
                    FeatureValue::Solid(solid) => solid,
                    _ => return Err(ir_error("REFERENCE_TYPE_MISMATCH", right_feature_id)),
                };
                FeatureValue::Solid(match mode {
                    BooleanMode::Union => left.union(right)?,
                    BooleanMode::Cut => left.cut(right)?,
                    BooleanMode::Intersect => left.intersect(right)?,
                })
            }
        };
        features.insert(feature.id.as_str(), value);
    }
    let output_key = resolved_key(&features, &body.source_feature_id)?;
    match features.remove(output_key.as_str()) {
        Some(FeatureValue::Solid(solid)) => Ok(solid),
        _ => Err(ir_error("INVALID_BODY", body.id.clone())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fillet_feature_rebuilds_from_its_parameter() {
        use crate::cad_ir::{Body, Feature, Parameter, SCHEMA_VERSION};
        let mut document = Document {
            schema_version: SCHEMA_VERSION,
            revision_id: "rounded_1".into(),
            parameters: vec![Parameter {
                id: "radius".into(),
                name: "Edge radius".into(),
                value_mm: 1.0,
            }],
            features: vec![
                Feature {
                    id: "profile".into(),
                    name: "Profile".into(),
                    suppressed: false,
                    operation: Operation::Rectangle {
                        width: Dimension::Literal { mm: 60.0 },
                        depth: Dimension::Literal { mm: 40.0 },
                    },
                },
                Feature {
                    id: "pad".into(),
                    name: "Pad".into(),
                    suppressed: false,
                    operation: Operation::Extrude {
                        sketch_id: "profile".into(),
                        distance: Dimension::Literal { mm: 10.0 },
                    },
                },
                Feature {
                    id: "rounded".into(),
                    name: "Rounded edges".into(),
                    suppressed: false,
                    operation: Operation::Fillet {
                        body_feature_id: "pad".into(),
                        radius: Dimension::Parameter {
                            parameter_id: "radius".into(),
                        },
                    },
                },
            ],
            bodies: vec![Body {
                id: "body".into(),
                name: "Plate".into(),
                source_feature_id: "rounded".into(),
            }],
        };
        let first = build_body(&document, "body").unwrap().volume_mm3();
        document.parameters[0].value_mm = 2.0;
        let second = build_body(&document, "body").unwrap().volume_mm3();
        assert!(second < first && first < 24000.0);
        document.features[2].suppressed = true;
        assert!((build_body(&document, "body").unwrap().volume_mm3() - 24000.0).abs() < 0.001);
        document.features[2].suppressed = false;
        document.parameters[0].value_mm = 1000.0;
        document.bodies[0].source_feature_id = "pad".into();
        assert!((build_body(&document, "body").unwrap().volume_mm3() - 24000.0).abs() < 0.001);
        document.bodies[0].source_feature_id = "rounded".into();
        assert!(build_body(&document, "body").is_err());
    }
    use crate::cad_ir::{Body, Feature, Parameter};

    #[test]
    fn parameter_edit_rebuilds_native_geometry() {
        let mut document = Document {
            schema_version: 2,
            revision_id: "revision_1".into(),
            parameters: vec![Parameter {
                id: "width".into(),
                name: "Width".into(),
                value_mm: 80.0,
            }],
            features: vec![
                Feature {
                    id: "sketch".into(),
                    name: "Profile".into(),
                    suppressed: false,
                    operation: Operation::Rectangle {
                        width: Dimension::Parameter {
                            parameter_id: "width".into(),
                        },
                        depth: Dimension::Literal { mm: 40.0 },
                    },
                },
                Feature {
                    id: "extrude".into(),
                    name: "Extrusion".into(),
                    suppressed: false,
                    operation: Operation::Extrude {
                        sketch_id: "sketch".into(),
                        distance: Dimension::Literal { mm: 10.0 },
                    },
                },
            ],
            bodies: vec![Body {
                id: "body".into(),
                name: "Plate".into(),
                source_feature_id: "extrude".into(),
            }],
        };
        assert!((build_body(&document, "body").unwrap().volume_mm3() - 32000.0).abs() < 0.001);
        document.parameters[0].value_mm = 120.0;
        assert!((build_body(&document, "body").unwrap().volume_mm3() - 48000.0).abs() < 0.001);
        document.features.push(Feature {
            id: "one_edge".into(),
            name: "One edge".into(),
            suppressed: false,
            operation: Operation::FilletEdge {
                body_feature_id: "extrude".into(),
                edge_key: "box-edge:x:ymin:zmin".into(),
                radius: Dimension::Literal { mm: 1.0 },
            },
        });
        document.bodies[0].source_feature_id = "one_edge".into();
        let first_edge_volume = build_body(&document, "body").unwrap().volume_mm3();
        assert!(first_edge_volume > 0.0 && first_edge_volume < 48000.0);
        document.parameters[0].value_mm = 90.0;
        let resized_edge_volume = build_body(&document, "body").unwrap().volume_mm3();
        assert!(resized_edge_volume > 0.0 && resized_edge_volume < 36000.0);
        if let Operation::FilletEdge { edge_key, .. } = &mut document.features[2].operation {
            *edge_key = "box-edge:x:ymin:zmiddle".into();
        }
        assert!(build_body(&document, "body").is_err());
        if let Operation::FilletEdge { edge_key, .. } = &mut document.features[2].operation {
            *edge_key = "box-edge:x:ymin:zmin".into();
        }
        assert_eq!(
            build_body(&document, "missing").err().unwrap().code,
            "MISSING_BODY"
        );
    }

    #[test]
    fn constrained_sketch_extrudes_on_workplanes_and_rebuilds_from_dimension() {
        let mut document: Document = serde_json::from_value(serde_json::json!({
            "schemaVersion": 2, "revisionId": "sketch_revision",
            "parameters": [{"id":"width","name":"Width","valueMm":20.0}],
            "features": [
                {"id":"profile","name":"Constrained profile","operation":{
                    "type":"sketch2d", "plane":"xy", "originMm":[5.0,6.0,7.0],
                    "points":[
                        {"id":"a","xMm":0.0,"yMm":0.0},
                        {"id":"b","xMm":19.0,"yMm":0.2},
                        {"id":"c","xMm":20.0,"yMm":10.0},
                        {"id":"d","xMm":0.0,"yMm":10.0}
                    ],
                    "lines":[
                        {"id":"ab","startPointId":"a","endPointId":"b"},
                        {"id":"bc","startPointId":"b","endPointId":"c"},
                        {"id":"cd","startPointId":"c","endPointId":"d"},
                        {"id":"da","startPointId":"d","endPointId":"a"}
                    ],
                    "constraints":[
                        {"kind":"fixed","id":"anchor","pointId":"a","xMm":0.0,"yMm":0.0},
                        {"kind":"horizontal","id":"h1","lineId":"ab"},
                        {"kind":"vertical","id":"v1","lineId":"bc"},
                        {"kind":"horizontal","id":"h2","lineId":"cd"},
                        {"kind":"vertical","id":"v2","lineId":"da"},
                        {"kind":"length","id":"w","lineId":"ab","distance":{"kind":"parameter","parameterId":"width"}},
                        {"kind":"length","id":"h","lineId":"bc","distance":{"kind":"literal","mm":10.0}}
                    ]
                }},
                {"id":"pad","name":"Pad","operation":{"type":"extrude","sketchId":"profile","distance":{"kind":"literal","mm":8.0}}}
            ],
            "bodies":[{"id":"body","name":"Body","sourceFeatureId":"pad"}]
        })).unwrap();
        assert!((build_body(&document, "body").unwrap().volume_mm3() - 1600.0).abs() < 0.01);
        document.parameters[0].value_mm = 30.0;
        assert!((build_body(&document, "body").unwrap().volume_mm3() - 2400.0).abs() < 0.01);
        if let Operation::Sketch2d { plane, .. } = &mut document.features[0].operation {
            *plane = crate::cad_ir::SketchPlane::Xz;
        }
        assert!((build_body(&document, "body").unwrap().volume_mm3() - 2400.0).abs() < 0.01);
        let mut unused = document.features[0].clone();
        unused.id = "unused_sketch".into();
        if let Operation::Sketch2d { constraints, .. } = &mut unused.operation {
            constraints.push(crate::cad_ir::SketchConstraint::Length {
                id: "conflicting_width".into(),
                line_id: "ab".into(),
                distance: Dimension::Literal { mm: 40.0 },
            });
        }
        document.features.push(unused);
        assert!((build_body(&document, "body").unwrap().volume_mm3() - 2400.0).abs() < 0.01);
    }
}
