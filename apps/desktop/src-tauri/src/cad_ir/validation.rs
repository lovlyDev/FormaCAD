use super::*;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FeatureKind {
    Datum,
    DatumPlane,
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
        let mut hole_sources = HashSet::new();
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
            let pattern_target = |target: &str, repeat_hole: bool| {
                reference(target, FeatureKind::Solid)?;
                if repeat_hole && !hole_sources.contains(target) {
                    return Err(ValidationError::new(
                        ErrorCode::InvalidValue,
                        Some(id.to_owned()),
                    ));
                }
                Ok(())
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
                Operation::Torus {
                    major_radius,
                    minor_radius,
                } => {
                    let major = resolve_dimension(major_radius, &parameters, id)?;
                    let minor = resolve_dimension(minor_radius, &parameters, id)?;
                    torus::validate_radii(major, minor, id)?;
                    FeatureKind::Solid
                }
                Operation::DatumPlane { .. }
                | Operation::DatumAxis { .. }
                | Operation::DatumPoint { .. } => {
                    feature.operation.validate_datum(id)?;
                    if matches!(feature.operation, Operation::DatumPlane { .. }) {
                        FeatureKind::DatumPlane
                    } else {
                        FeatureKind::Datum
                    }
                }
                Operation::DatumOnReference { geometry, anchor } => {
                    use super::datum_anchor::{DatumGeometry, DatumSource};
                    if !anchor.valid() || !geometry.valid() {
                        return Err(ValidationError::new(
                            ErrorCode::InvalidValue,
                            Some(id.to_owned()),
                        ));
                    }
                    match &anchor.source {
                        DatumSource::Plane { feature_id } => {
                            reference(feature_id, FeatureKind::DatumPlane)?
                        }
                        DatumSource::Topology {
                            reference: topology,
                            ..
                        } => {
                            reference(&topology.owner_feature_id, FeatureKind::Solid)?;
                            for occurrence in &topology.occurrence_path {
                                reference(occurrence, FeatureKind::Solid)?;
                            }
                        }
                    }
                    if matches!(geometry, DatumGeometry::Plane { .. }) {
                        FeatureKind::DatumPlane
                    } else {
                        FeatureKind::Datum
                    }
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
                Operation::OffsetProfile {
                    sketch_id, offset, ..
                } => {
                    reference(sketch_id, FeatureKind::Sketch)?;
                    check_dimension(offset, &parameters, true, id)?;
                    FeatureKind::ProfileSketch
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
                    additional_references,
                    radius,
                }
                | Operation::ChamferReferencedEdge {
                    body_feature_id,
                    reference: topology,
                    additional_references,
                    distance: radius,
                } => {
                    reference(body_feature_id, FeatureKind::Solid)?;
                    check_dimension(radius, &parameters, false, id)?;
                    if !super::topology_refs::valid_blend_edges(topology, additional_references) {
                        return Err(ValidationError::new(
                            ErrorCode::InvalidValue,
                            Some(id.to_owned()),
                        ));
                    }
                    for edge in std::iter::once(topology).chain(additional_references) {
                        reference(&edge.owner_feature_id, FeatureKind::Solid)?;
                        for occurrence in &edge.occurrence_path {
                            reference(occurrence, FeatureKind::Solid)?;
                        }
                    }
                    FeatureKind::Solid
                }
                Operation::ExtrudeFace {
                    body_feature_id,
                    reference: topology,
                    distance,
                }
                | Operation::ThickenFace {
                    body_feature_id,
                    reference: topology,
                    offset: distance,
                } => {
                    reference(body_feature_id, FeatureKind::Solid)?;
                    let distance = resolve_dimension(distance, &parameters, id)?;
                    if !topology.is_valid()
                        || topology.kind != TopologyKind::Face
                        || !distance.is_finite()
                        || distance.abs() <= 1e-7
                        || distance.abs() > MAX_DIMENSION_MM
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
                Operation::Shell {
                    body_feature_id,
                    openings,
                    offset,
                } => {
                    reference(body_feature_id, FeatureKind::Solid)?;
                    let offset = resolve_dimension(offset, &parameters, id)?;
                    if !super::shell::valid_openings(openings)
                        || !offset.is_finite()
                        || offset.abs() <= 1e-7
                        || offset.abs() > MAX_DIMENSION_MM
                    {
                        return Err(ValidationError::new(
                            ErrorCode::InvalidValue,
                            Some(id.to_owned()),
                        ));
                    }
                    let topology = &openings[0];
                    reference(&topology.owner_feature_id, FeatureKind::Solid)?;
                    for occurrence in &topology.occurrence_path {
                        reference(occurrence, FeatureKind::Solid)?;
                    }
                    FeatureKind::Solid
                }
                Operation::ExtractSolid {
                    body_feature_id,
                    index,
                    expected_count,
                } => {
                    reference(body_feature_id, FeatureKind::Solid)?;
                    if !(2..=128).contains(expected_count) || index >= expected_count {
                        return Err(ValidationError::new(
                            ErrorCode::InvalidValue,
                            Some(id.to_owned()),
                        ));
                    }
                    FeatureKind::Solid
                }
                Operation::PatternLinear {
                    body_feature_id,
                    repeat_hole,
                    layout,
                } => {
                    pattern_target(body_feature_id, *repeat_hole)?;
                    layout.validate(id)?;
                    FeatureKind::Solid
                }
                Operation::PatternCircular {
                    body_feature_id,
                    repeat_hole,
                    layout,
                } => {
                    pattern_target(body_feature_id, *repeat_hole)?;
                    layout.validate(id)?;
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
                Operation::Transform {
                    body_feature_id,
                    matrix,
                } => {
                    reference(body_feature_id, FeatureKind::Solid)?;
                    super::affine_transform::validate(matrix, id)?;
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
                Operation::MirrorFeatures {
                    body_feature_id,
                    base_feature_id,
                    feature_ids,
                    plane_origin_mm,
                    plane_normal,
                } => {
                    reference(body_feature_id, FeatureKind::Solid)?;
                    if let Some(base) = base_feature_id {
                        reference(base, FeatureKind::Solid)?;
                    }
                    for selected in feature_ids {
                        reference(selected, FeatureKind::Solid)?;
                    }
                    super::transforms::validate_axis(*plane_origin_mm, *plane_normal, id)?;
                    if !super::mirror_features::valid_range(
                        &self.features,
                        base_feature_id.as_deref(),
                        body_feature_id,
                        feature_ids,
                    ) {
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
                }
                | Operation::SplitPlane {
                    body_feature_id,
                    plane_origin_mm,
                    plane_normal,
                    ..
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
            if !feature.suppressed && matches!(feature.operation, Operation::Hole { .. }) {
                hole_sources.insert(id);
            }
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
        for group in &self.groups {
            check_identity(&mut ids, &group.id)?;
            check_name(&group.name, &group.id)?;
        }
        super::groups::validate(&self.groups, &self.bodies)?;
        for feature in &self.features {
            if let Operation::DatumOnReference { anchor, .. } = &feature.operation {
                if let super::datum_anchor::DatumSource::Topology { body_id, .. } = &anchor.source {
                    if !self.bodies.iter().any(|body| &body.id == body_id) {
                        return Err(ValidationError::new(
                            ErrorCode::BrokenReference,
                            Some(feature.id.clone()),
                        ));
                    }
                }
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
