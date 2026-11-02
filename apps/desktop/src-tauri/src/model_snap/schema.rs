use crate::cad_ir::{TopologyKind, TopologyReference};
pub use crate::model_measurement::schema::{
    ImportedAssetSeal, MeasurementBinding as SnapBinding, MeasurementEngine as SnapEngine,
    MAX_DOCUMENT_BYTES, MAX_REPORT_BYTES,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum SnapQuery {
    EdgeMidpoint {
        reference: TopologyReference,
    },
    CircleCenter {
        reference: TopologyReference,
    },
    Vertex {
        reference: TopologyReference,
    },
    Face {
        reference: TopologyReference,
        ray_origin_mm: [f64; 3],
        ray_direction: [f64; 3],
    },
}
impl SnapQuery {
    pub fn reference(&self) -> &TopologyReference {
        match self {
            Self::EdgeMidpoint { reference }
            | Self::CircleCenter { reference }
            | Self::Vertex { reference }
            | Self::Face { reference, .. } => reference,
        }
    }
    pub fn tag(&self) -> i32 {
        match self {
            Self::EdgeMidpoint { .. } => 0,
            Self::CircleCenter { .. } => 1,
            Self::Face { .. } => 2,
            Self::Vertex { .. } => 3,
        }
    }
    pub fn ray(&self) -> Option<[f64; 6]> {
        match self {
            Self::Face {
                ray_origin_mm: o,
                ray_direction: d,
                ..
            } => Some([o[0], o[1], o[2], d[0], d[1], d[2]]),
            _ => None,
        }
    }
    pub fn valid(&self) -> bool {
        let reference = self.reference();
        if !reference.is_valid() {
            return false;
        }
        match self {
            Self::EdgeMidpoint { .. } | Self::CircleCenter { .. } => {
                reference.kind == TopologyKind::Edge
            }
            Self::Vertex { .. } => reference.kind == TopologyKind::Vertex,
            Self::Face {
                ray_origin_mm,
                ray_direction,
                ..
            } => {
                reference.kind == TopologyKind::Face
                    && ray_origin_mm
                        .iter()
                        .all(|value| value.is_finite() && value.abs() <= 1e9)
                    && ray_direction
                        .iter()
                        .all(|value| value.is_finite() && value.abs() <= 1e6)
                    && ray_direction.iter().map(|value| value * value).sum::<f64>() >= 1e-18
            }
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SnapValue {
    pub point_mm: [f64; 3],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub normal: Option<[f64; 3]>,
}
impl SnapValue {
    pub fn valid_for(&self, query: &SnapQuery) -> bool {
        self.point_mm
            .iter()
            .all(|value| value.is_finite() && value.abs() <= 1e9)
            && (!matches!(query, SnapQuery::Face { .. }) || self.normal.is_some())
            && self.normal.as_ref().is_none_or(|normal| {
                normal.iter().all(|value| value.is_finite())
                    && (normal.iter().map(|value| value * value).sum::<f64>() - 1.).abs() <= 1e-8
            })
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SnapReport {
    pub schema_version: u32,
    pub request_id: String,
    pub engine: SnapEngine,
    pub evaluation_source: String,
    pub revision_id: String,
    pub body_id: String,
    pub source_sha256: String,
    pub source_size: u64,
    pub document_sha256: String,
    pub imported_asset_seals: Vec<ImportedAssetSeal>,
    pub query: SnapQuery,
    pub result: SnapValue,
}
impl SnapReport {
    pub fn matches(&self, request_id: &str, binding: &SnapBinding, query: &SnapQuery) -> bool {
        binding.valid()
            && uuid::Uuid::parse_str(request_id).is_ok()
            && request_id.len() == 36
            && self.schema_version == 1
            && self.request_id == request_id
            && self.engine.name == "occt"
            && self.engine.protocol_version == 1
            && self.evaluation_source == "rebuiltAuthoredBody"
            && self.revision_id == binding.revision_id
            && self.body_id == binding.body_id
            && self.source_sha256 == binding.source_sha256
            && self.source_size == binding.source_size
            && self.document_sha256 == binding.document_sha256
            && self.imported_asset_seals == binding.imported_asset_seals
            && self.query == *query
            && query.valid()
            && self.result.valid_for(query)
    }
}
