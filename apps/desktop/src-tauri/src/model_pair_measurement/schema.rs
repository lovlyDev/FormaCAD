//! Two exact references on ONE captured authored body; no renderer point/vector authority.
use crate::{
    cad_ir::{TopologyKind, TopologyReference},
    model_measurement::schema::{
        ImportedAssetSeal, MeasurementBinding, MeasurementEngine, MAX_COORDINATE_MM,
    },
};
use serde::{Deserialize, Serialize};
pub const MAX_PAIR_REPORT_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum PairMeasurementQuery {
    MinimumDistance {
        first: TopologyReference,
        second: TopologyReference,
    },
    FaceNormalAngle {
        first: TopologyReference,
        second: TopologyReference,
    },
    EdgeAcuteAngle {
        first: TopologyReference,
        second: TopologyReference,
    },
}
impl PairMeasurementQuery {
    pub fn references(&self) -> (&TopologyReference, &TopologyReference) {
        match self {
            Self::MinimumDistance { first, second }
            | Self::FaceNormalAngle { first, second }
            | Self::EdgeAcuteAngle { first, second } => (first, second),
        }
    }
    pub fn tag(&self) -> i32 {
        match self {
            Self::MinimumDistance { .. } => 0,
            Self::FaceNormalAngle { .. } => 1,
            Self::EdgeAcuteAngle { .. } => 2,
        }
    }
    pub fn valid(&self) -> bool {
        let (first, second) = self.references();
        first.is_valid()
            && second.is_valid()
            && match self {
                Self::MinimumDistance { .. } => true,
                Self::FaceNormalAngle { .. } => {
                    first.kind == TopologyKind::Face && second.kind == TopologyKind::Face
                }
                Self::EdgeAcuteAngle { .. } => {
                    first.kind == TopologyKind::Edge && second.kind == TopologyKind::Edge
                }
            }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum PairMeasurementValue {
    MinimumDistance {
        distance_mm: f64,
        point_a_mm: [f64; 3],
        point_b_mm: [f64; 3],
    },
    FaceNormalAngle {
        angle_deg: f64,
    },
    EdgeAcuteAngle {
        angle_deg: f64,
    },
}
impl PairMeasurementValue {
    pub fn valid_for(&self, query: &PairMeasurementQuery) -> bool {
        if !query.valid() {
            return false;
        }
        match (self, query) {
            (
                Self::MinimumDistance {
                    distance_mm,
                    point_a_mm,
                    point_b_mm,
                },
                PairMeasurementQuery::MinimumDistance { .. },
            ) => {
                if !distance_mm.is_finite()
                    || *distance_mm < 0.
                    || !point_a_mm
                        .iter()
                        .chain(point_b_mm)
                        .all(|v| v.is_finite() && v.abs() <= MAX_COORDINATE_MM)
                {
                    return false;
                }
                let separation = point_a_mm
                    .iter()
                    .zip(point_b_mm)
                    .map(|(a, b)| (a - b).powi(2))
                    .sum::<f64>()
                    .sqrt();
                separation.is_finite()
                    && (separation - distance_mm).abs() <= 1e-6_f64.max(distance_mm * 1e-9)
            }
            (Self::FaceNormalAngle { angle_deg }, PairMeasurementQuery::FaceNormalAngle { .. }) => {
                angle_deg.is_finite() && (0. ..=180.).contains(angle_deg)
            }
            (Self::EdgeAcuteAngle { angle_deg }, PairMeasurementQuery::EdgeAcuteAngle { .. }) => {
                angle_deg.is_finite() && (0. ..=90.).contains(angle_deg)
            }
            _ => false,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PairMeasurementReport {
    pub schema_version: u32,
    pub request_id: String,
    pub engine: MeasurementEngine,
    pub evaluation_source: String,
    pub revision_id: String,
    pub body_id: String,
    pub source_sha256: String,
    pub source_size: u64,
    pub document_sha256: String,
    pub imported_asset_seals: Vec<ImportedAssetSeal>,
    pub query: PairMeasurementQuery,
    pub result: PairMeasurementValue,
}
impl PairMeasurementReport {
    pub fn matches(
        &self,
        id: &str,
        binding: &MeasurementBinding,
        query: &PairMeasurementQuery,
    ) -> bool {
        binding.valid()
            && query.valid()
            && uuid::Uuid::parse_str(id).is_ok()
            && id.len() == 36
            && self.schema_version == 1
            && self.request_id == id
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
            && self.result.valid_for(query)
    }
}
#[cfg(test)]
#[path = "schema_tests.rs"]
mod tests;
