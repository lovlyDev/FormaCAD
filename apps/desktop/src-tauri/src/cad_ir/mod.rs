//! CAD IR v2 foundation. This module validates intent and dependency structure;
//! geometric execution and topology validation belong to the native CAD worker.
use serde::{Deserialize, Serialize};
pub mod assets;
pub mod topology_refs;
pub use topology_refs::{TopologyKind, TopologyReference};

pub const SCHEMA_VERSION: u32 = 2;
fn is_false(value: &bool) -> bool {
    !*value
}
const MAX_FEATURES: usize = 4096;
const MAX_PARAMETERS: usize = 10000;
const MAX_BODIES: usize = 4096;
const MAX_DIMENSION_MM: f64 = 10000.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    InvalidDocument,
    InvalidId,
    DuplicateId,
    InvalidValue,
    BrokenReference,
    ReferenceTypeMismatch,
    InvalidBody,
    InvalidSketch,
    SketchConstraintConflict,
    UnsupportedSuppression,
    MissingTarget,
    UnsupportedField,
    ParameterBound,
    RevisionConflict,
    NoCommands,
    NoChange,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, thiserror::Error)]
#[error("{code:?}")]
pub struct ValidationError {
    pub code: ErrorCode,
    pub target_id: Option<String>,
}

impl ValidationError {
    fn new(code: ErrorCode, target_id: impl Into<Option<String>>) -> Self {
        Self {
            code,
            target_id: target_id.into(),
        }
    }
}

type Result<T> = std::result::Result<T, ValidationError>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Document {
    pub schema_version: u32,
    pub revision_id: String,
    pub parameters: Vec<Parameter>,
    pub features: Vec<Feature>,
    pub bodies: Vec<Body>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Parameter {
    pub id: String,
    pub name: String,
    /// Canonical length value. Display and input units are handled at the boundary.
    pub value_mm: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Feature {
    pub id: String,
    pub name: String,
    pub operation: Operation,
    #[serde(default, skip_serializing_if = "is_false")]
    pub suppressed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Body {
    pub id: String,
    pub name: String,
    pub source_feature_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Dimension {
    Literal { mm: f64 },
    Parameter { parameter_id: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Operation {
    ImportStep {
        asset_id: String,
        sha256: String,
    },
    Sketch2d {
        plane: SketchPlane,
        origin_mm: [f64; 3],
        points: Vec<SketchPoint>,
        lines: Vec<SketchLine>,
        constraints: Vec<SketchConstraint>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        bindings: Vec<sketch::bindings::CoordinateBinding>,
    },
    Rectangle {
        width: Dimension,
        depth: Dimension,
    },
    Circle {
        radius: Dimension,
    },
    Sphere {
        radius: Dimension,
    },
    Cone {
        bottom_radius: Dimension,
        top_radius: Dimension,
        height: Dimension,
    },
    Extrude {
        sketch_id: String,
        distance: Dimension,
    },
    Revolve {
        sketch_id: String,
        axis_origin_mm: [f64; 3],
        axis_direction: [f64; 3],
        angle_deg: f64,
    },
    Hole {
        body_feature_id: String,
        center_mm: [f64; 2],
        start_z_mm: f64,
        radius: Dimension,
        depth: Dimension,
    },
    Fillet {
        body_feature_id: String,
        radius: Dimension,
    },
    FilletEdge {
        body_feature_id: String,
        edge_key: String,
        radius: Dimension,
    },
    FilletReferencedEdge {
        body_feature_id: String,
        reference: TopologyReference,
        radius: Dimension,
    },
    Chamfer {
        body_feature_id: String,
        distance: Dimension,
    },
    Translate {
        body_feature_id: String,
        offset_mm: [f64; 3],
    },
    Rotate {
        body_feature_id: String,
        axis_origin_mm: [f64; 3],
        axis_direction: [f64; 3],
        angle_deg: f64,
    },
    Mirror {
        body_feature_id: String,
        plane_origin_mm: [f64; 3],
        plane_normal: [f64; 3],
    },
    Boolean {
        left_feature_id: String,
        right_feature_id: String,
        mode: BooleanMode,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SketchPlane {
    Xy,
    Xz,
    Yz,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SketchPoint {
    pub id: String,
    pub x_mm: f64,
    pub y_mm: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SketchLine {
    pub id: String,
    pub start_point_id: String,
    pub end_point_id: String,
    /// Participates in constraints but never contributes to the extrusion outline.
    #[serde(default)]
    pub construction: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum SketchConstraint {
    Fixed {
        id: String,
        point_id: String,
        x_mm: f64,
        y_mm: f64,
    },
    Horizontal {
        id: String,
        line_id: String,
    },
    Vertical {
        id: String,
        line_id: String,
    },
    Length {
        id: String,
        line_id: String,
        distance: Dimension,
    },
    Coincident {
        id: String,
        first_point_id: String,
        second_point_id: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BooleanMode {
    Union,
    Cut,
    Intersect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DimensionField {
    Width,
    Depth,
    Radius,
    BottomRadius,
    TopRadius,
    Height,
    Distance,
    HoleDepth,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "command",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Command {
    SetTopologyReference {
        feature_id: String,
        reference: TopologyReference,
    },
    AddParameter {
        parameter: Parameter,
    },
    AddFeature {
        feature: Feature,
    },
    AddBody {
        body: Body,
    },
    SetBodySource {
        body_id: String,
        source_feature_id: String,
    },
    SetParameter {
        parameter_id: String,
        value_mm: f64,
    },
    SetLiteral {
        feature_id: String,
        field: DimensionField,
        value_mm: f64,
    },
    RenameFeature {
        feature_id: String,
        name: String,
    },
    SetFeatureSuppressed {
        feature_id: String,
        suppressed: bool,
    },
    SetRotation {
        feature_id: String,
        axis_origin_mm: [f64; 3],
        axis_direction: [f64; 3],
        angle_deg: f64,
    },
    SetRevolution {
        feature_id: String,
        axis_origin_mm: [f64; 3],
        axis_direction: [f64; 3],
        angle_deg: f64,
    },
    SetMirrorPlane {
        feature_id: String,
        plane_origin_mm: [f64; 3],
        plane_normal: [f64; 3],
    },
    AddSketchBinding {
        feature_id: String,
        binding: sketch::bindings::CoordinateBinding,
    },
    SetSketchBinding {
        feature_id: String,
        binding: sketch::bindings::CoordinateBinding,
    },
    RemoveSketchBinding {
        feature_id: String,
        binding_id: String,
    },
    SetSketchPoint {
        feature_id: String,
        point_id: String,
        x_mm: f64,
        y_mm: f64,
    },
    SetSketchPlane {
        feature_id: String,
        plane: SketchPlane,
        origin_mm: [f64; 3],
    },
    SetSketchLength {
        feature_id: String,
        constraint_id: String,
        distance_mm: f64,
    },
    AddSketchConstraint {
        feature_id: String,
        constraint: SketchConstraint,
    },
    RemoveSketchConstraint {
        feature_id: String,
        constraint_id: String,
    },
}

mod commands;
mod dependencies;
mod diff;
pub mod history;
mod legacy;
pub mod sketch;
#[cfg(test)]
mod tests;
pub(crate) mod topology;
mod transforms;
mod validation;

pub use commands::apply_commands;
pub use dependencies::required_features;
pub use diff::{diff_documents, DocumentDiff, EntityChange, Snapshot};
pub use legacy::compile_json;
pub(crate) fn app_error(error: ValidationError) -> crate::core::AppError {
    let message = match error.code {
        ErrorCode::InvalidSketch => "Sketch profile is invalid",
        ErrorCode::SketchConstraintConflict => "Sketch constraints conflict",
        _ => "CAD document is invalid",
    };
    crate::core::AppError::Invalid(
        serde_json::json!({"code": error.code, "targetId": error.target_id, "message": message})
            .to_string(),
    )
}
