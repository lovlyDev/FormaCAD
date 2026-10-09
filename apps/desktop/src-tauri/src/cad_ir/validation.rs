use super::*;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FeatureKind {
    Sketch,
    ProfileSketch,
    Solid,
}

impl Document {
    /// Check structure before any CAD worker call. This does not validate BREP.
    pub fn validate(&self) -> Result<()> {
        super::assets::references(self)?;
        if self.schema_version != SCHEMA_VERSION
            || !valid_id(&self.revision_id)
            || self.features.len() > MAX_FEATURES
            || self.parameters.len() > MAX_PARAMETERS
            || self.bodies.len() > MAX_BODIES
        {
            return Err(ValidationError::new(ErrorCode::InvalidDocument, None));
        }
        let mut ids = HashSet::new();
        let mut parameters = HashMap::new();
        for parameter in &self.parameters {
            check_identity(&mut ids, &parameter.id)?;
            check_name(&parameter.name, &parameter.id)?;
            if !parameter.value_mm.is_finite() || parameter.value_mm.abs() > MAX_DIMENSION_MM {
                return Err(ValidationError::new(
                    ErrorCode::InvalidValue,
                    Some(parameter.id.clone()),
                ));
            }
            parameters.insert(parameter.id.as_str(), parameter.value_mm);
        }
        let mut features = HashMap::new();
        let mut sketch_points_total = 0usize;
        let mut sketch_constraints_total = 0usize;
        for feature in &self.features {
            check_identity(&mut ids, &feature.id)?;
            check_name(&feature.name, &feature.id)?;
            let id = feature.id.as_str();
            if feature.suppressed && feature.operation.suppression_source().is_none() {
                return Err(ValidationError::new(
                    ErrorCode::UnsupportedSuppression,
                    Some(id.to_owned()),
                ));
            }
            let reference = |target: &str, expected: FeatureKind| -> Result<()> {
                match features.get(target) {
                    Some(actual)
                        if *actual == expected
                            || (*actual == FeatureKind::ProfileSketch
                                && expected == FeatureKind::Sketch) =>
                    {
                        Ok(())
                    }
                    Some(_) => Err(ValidationError::new(
                        ErrorCode::ReferenceTypeMismatch,
                        Some(id.to_owned()),
                    )),
                    None => Err(ValidationError::new(
                        ErrorCode::BrokenReference,
                        Some(id.to_owned()),
                    )),
                }
            };
            let kind = match &feature.operation {
                Operation::ImportStep { asset_id, sha256 } => {
                    if !super::assets::valid_reference(asset_id, sha256) {
                        return Err(ValidationError::new(
                            ErrorCode::InvalidValue,
                            Some(id.to_owned()),
                        ));
                    }
                    FeatureKind::Solid
                }
                Operation::Sketch2d {
                    origin_mm,
                    points,
                    lines,
                    constraints,
                    bindings,
                    ..
                } => {
                    sketch_points_total += points.len();
                    sketch_constraints_total += constraints.len() + bindings.len();
                    if sketch_points_total > 512 || sketch_constraints_total > 1024 {
                        return Err(ValidationError::new(
                            ErrorCode::InvalidDocument,
                            Some(id.to_owned()),
                        ));
                    }
                    let (origin_mm, points) = super::sketch::bindings::resolve(
                        *origin_mm,
                        points,
                        constraints,
                        bindings,
                        &parameters,
                        id,
                    )?;
                    super::sketch::validate_structure(
                        origin_mm,
                        &points,
                        lines,
                        constraints,
                        &parameters,
                        id,
                    )?;
                    FeatureKind::ProfileSketch
                }
                Operation::Rectangle { width, depth } => {
                    check_dimension(width, &parameters, false, id)?;
                    check_dimension(depth, &parameters, false, id)?;
                    FeatureKind::Sketch
                }
                Operation::Circle { radius } => {
                    check_dimension(radius, &parameters, false, id)?;
                    FeatureKind::Sketch
                }
                Operation::Sphere { radius } => {
                    check_dimension(radius, &parameters, false, id)?;
                    FeatureKind::Solid
                }
                Operation::Cone {
                    bottom_radius,
                    top_radius,
                    height,
                } => {
                    check_dimension(bottom_radius, &parameters, false, id)?;
                    let top = resolve_dimension(top_radius, &parameters, id)?;
                    let bottom = resolve_dimension(bottom_radius, &parameters, id)?;
                    if !top.is_finite() || !(0.0..=MAX_DIMENSION_MM).contains(&top) || top == bottom
                    {
                        return Err(ValidationError::new(
                            ErrorCode::InvalidValue,
                            Some(id.to_owned()),
                        ));
                    }
                    check_dimension(height, &parameters, false, id)?;
                    FeatureKind::Solid
                }
                Operation::Extrude {
                    sketch_id,
                    distance,
                } => {
                    reference(sketch_id, FeatureKind::Sketch)?;
                    check_dimension(distance, &parameters, true, id)?;
                    FeatureKind::Solid
                }
                Operation::Revolve {
                    sketch_id,
                    axis_origin_mm,
                    axis_direction,
                    angle_deg,
                } => {
                    reference(sketch_id, FeatureKind::ProfileSketch)?;
                    super::transforms::validate_axis(*axis_origin_mm, *axis_direction, id)?;
                    if !angle_deg.is_finite() || *angle_deg == 0.0 || angle_deg.abs() > 360.0 {
                        return Err(ValidationError::new(
                            ErrorCode::InvalidValue,
                            Some(id.to_owned()),
                        ));
                    }
                    FeatureKind::Solid
                }
                Operation::Hole {
                    body_feature_id,
                    center_mm,
                    start_z_mm,
                    radius,
                    depth,
                } => {
                    reference(body_feature_id, FeatureKind::Solid)?;
                    check_dimension(radius, &parameters, false, id)?;
                    check_dimension(depth, &parameters, false, id)?;
                    if center_mm
                        .iter()
                        .chain(std::iter::once(start_z_mm))
                        .any(|value| !value.is_finite() || value.abs() > MAX_DIMENSION_MM)
                    {
                        return Err(ValidationError::new(
                            ErrorCode::InvalidValue,
                            Some(id.to_owned()),
                        ));
                    }
                    FeatureKind::Solid
                }
                Operation::Fillet {
                    body_feature_id,
                    radius,
                } => {
                    reference(body_feature_id, FeatureKind::Solid)?;
                    check_dimension(radius, &parameters, false, id)?;
                    FeatureKind::Solid
                }
                Operation::FilletEdge {
                    body_feature_id,
                    edge_key,
                    radius,
                } => {
                    reference(body_feature_id, FeatureKind::Solid)?;
                    check_dimension(radius, &parameters, false, id)?;
                    if !super::topology::valid_box_edge_key(edge_key) {
                        return Err(ValidationError::new(
                            ErrorCode::InvalidValue,
                            Some(id.to_owned()),
                        ));
                    }
                    FeatureKind::Solid
                }
                Operation::Chamfer {
                    body_feature_id,
                    distance,
                } => {
                    reference(body_feature_id, FeatureKind::Solid)?;
                    check_dimension(distance, &parameters, false, id)?;
                    FeatureKind::Solid
                }
                Operation::FilletReferencedEdge {
                    body_feature_id,
                    reference: topology,
                    radius,
                } => {
                    reference(body_feature_id, FeatureKind::Solid)?;
                    check_dimension(radius, &parameters, false, id)?;
                    if !topology.is_valid()
                        || topology.kind != TopologyKind::Edge
                        || !super::topology::valid_box_edge_key(&topology.role)
                    {
                        return Err(ValidationError::new(
                            ErrorCode::InvalidValue,
                            Some(id.to_owned()),
                        ));
                    }
                    reference(&topology.owner_feature_id, FeatureKind::Solid)?;
                    for occurrence in &topology.occurrence_path {
                        reference(occurrence, FeatureKind::Solid)?;
                    }
                    FeatureKind::Solid
                }
                Operation::Translate {
                    body_feature_id,
                    offset_mm,
                } => {
                    reference(body_feature_id, FeatureKind::Solid)?;
                    if offset_mm
                        .iter()
                        .any(|value| !value.is_finite() || value.abs() > MAX_DIMENSION_MM)
                    {
                        return Err(ValidationError::new(
                            ErrorCode::InvalidValue,
                            Some(id.to_owned()),
                        ));
                    }
                    FeatureKind::Solid
                }
                Operation::Rotate {
                    body_feature_id,
                    axis_origin_mm,
                    axis_direction,
                    angle_deg,
                } => {
                    reference(body_feature_id, FeatureKind::Solid)?;
                    super::transforms::validate_axis(*axis_origin_mm, *axis_direction, id)?;
                    if !angle_deg.is_finite() || angle_deg.abs() > 360.0 {
                        return Err(ValidationError::new(
                            ErrorCode::InvalidValue,
                            Some(id.to_owned()),
                        ));
                    }
                    FeatureKind::Solid
                }
                Operation::Mirror {
                    body_feature_id,
                    plane_origin_mm,
                    plane_normal,
                } => {
                    reference(body_feature_id, FeatureKind::Solid)?;
                    super::transforms::validate_axis(*plane_origin_mm, *plane_normal, id)?;
                    FeatureKind::Solid
                }
                Operation::Boolean {
                    left_feature_id,
                    right_feature_id,
                    ..
                } => {
                    reference(left_feature_id, FeatureKind::Solid)?;
                    reference(right_feature_id, FeatureKind::Solid)?;
                    if left_feature_id == right_feature_id {
                        return Err(ValidationError::new(
                            ErrorCode::InvalidValue,
                            Some(id.to_owned()),
                        ));
                    }
                    FeatureKind::Solid
                }
            };
            features.insert(id, kind);
        }
        for body in &self.bodies {
            check_identity(&mut ids, &body.id)?;
            check_name(&body.name, &body.id)?;
            if features.get(body.source_feature_id.as_str()) != Some(&FeatureKind::Solid) {
                return Err(ValidationError::new(
                    ErrorCode::InvalidBody,
                    Some(body.id.clone()),
                ));
            }
        }
        Ok(())
    }
}

pub(super) fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 80
        && id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
}

fn check_identity(ids: &mut HashSet<String>, id: &str) -> Result<()> {
    if !valid_id(id) {
        return Err(ValidationError::new(
            ErrorCode::InvalidId,
            Some(id.to_owned()),
        ));
    }
    if !ids.insert(id.to_owned()) {
        return Err(ValidationError::new(
            ErrorCode::DuplicateId,
            Some(id.to_owned()),
        ));
    }
    Ok(())
}

fn check_name(name: &str, id: &str) -> Result<()> {
    if name.trim().is_empty() || name.chars().count() > 120 || name.chars().any(char::is_control) {
        return Err(ValidationError::new(
            ErrorCode::InvalidValue,
            Some(id.to_owned()),
        ));
    }
    Ok(())
}

fn valid_dimension(value: f64, signed: bool) -> bool {
    value.is_finite() && value != 0.0 && value.abs() <= MAX_DIMENSION_MM && (signed || value > 0.0)
}

fn check_dimension(
    value: &Dimension,
    parameters: &HashMap<&str, f64>,
    signed: bool,
    id: &str,
) -> Result<()> {
    let resolved = resolve_dimension(value, parameters, id)?;
    if !valid_dimension(resolved, signed) {
        return Err(ValidationError::new(
            ErrorCode::InvalidValue,
            Some(id.to_owned()),
        ));
    }
    Ok(())
}

fn resolve_dimension(value: &Dimension, parameters: &HashMap<&str, f64>, id: &str) -> Result<f64> {
    let resolved = match value {
        Dimension::Literal { mm } => *mm,
        Dimension::Parameter { parameter_id } => *parameters
            .get(parameter_id.as_str())
            .ok_or_else(|| ValidationError::new(ErrorCode::BrokenReference, Some(id.to_owned())))?,
    };
    Ok(resolved)
}
