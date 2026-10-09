//! Shared, native-independent exact measurement wire contract. No mesh ordinals.
use crate::cad_ir::{TopologyKind, TopologyReference};
use serde::{Deserialize, Serialize};

pub const MAX_DOCUMENT_BYTES: usize = 60_000;
pub const MAX_REPORT_BYTES: usize = 64 * 1024;
pub const MAX_COORDINATE_MM: f64 = 1_000_000_000.;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum MeasurementQuery {
    // Empty struct keeps serde's unknown-field check; a unit variant discards
    // its remaining map fields even with deny_unknown_fields on the enum.
    BodyMetrics {},
    EdgeLength { reference: TopologyReference },
    EdgeRadius { reference: TopologyReference },
    EdgeDiameter { reference: TopologyReference },
    FaceArea { reference: TopologyReference },
    PlanarFace { reference: TopologyReference },
}

impl MeasurementQuery {
    pub fn reference(&self) -> Option<&TopologyReference> {
        match self {
            Self::BodyMetrics {} => None,
            Self::EdgeLength { reference }
            | Self::EdgeRadius { reference }
            | Self::EdgeDiameter { reference }
            | Self::FaceArea { reference }
            | Self::PlanarFace { reference } => Some(reference),
        }
    }
    pub fn tag(&self) -> i32 {
        match self {
            Self::BodyMetrics {} => 0,
            Self::EdgeLength { .. } => 1,
            Self::EdgeRadius { .. } => 2,
            Self::FaceArea { .. } => 3,
            Self::PlanarFace { .. } => 4,
            Self::EdgeDiameter { .. } => 5,
        }
    }
    pub fn valid(&self) -> bool {
        self.reference().is_none_or(|reference| {
            reference.is_valid()
                && reference.kind
                    == match self {
                        Self::EdgeLength { .. }
                        | Self::EdgeRadius { .. }
                        | Self::EdgeDiameter { .. } => TopologyKind::Edge,
                        Self::FaceArea { .. } | Self::PlanarFace { .. } => TopologyKind::Face,
                        Self::BodyMetrics {} => return false,
                    }
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum MeasurementValue {
    BodyMetrics {
        volume_mm3: f64,
        area_mm2: f64,
        face_count: u32,
        edge_count: u32,
        extents_mm: [f64; 3],
    },
    EdgeLength {
        length_mm: f64,
    },
    EdgeRadius {
        radius_mm: f64,
    },
    EdgeDiameter {
        diameter_mm: f64,
    },
    FaceArea {
        area_mm2: f64,
    },
    PlanarFace {
        area_mm2: f64,
        origin_mm: [f64; 3],
        normal: [f64; 3],
    },
}

impl MeasurementValue {
    pub fn valid_for(&self, query: &MeasurementQuery) -> bool {
        let positive = |value: f64| value.is_finite() && value > 0.;
        match (self, query) {
            (
                Self::BodyMetrics {
                    volume_mm3,
                    area_mm2,
                    face_count,
                    edge_count,
                    extents_mm,
                },
                MeasurementQuery::BodyMetrics {},
            ) => {
                positive(*volume_mm3)
                    && positive(*area_mm2)
                    && *face_count > 0
                    && *edge_count > 0
                    && extents_mm
                        .iter()
                        .all(|v| positive(*v) && *v <= MAX_COORDINATE_MM)
            }
            (Self::EdgeLength { length_mm }, MeasurementQuery::EdgeLength { .. }) => {
                positive(*length_mm)
            }
            (Self::EdgeRadius { radius_mm }, MeasurementQuery::EdgeRadius { .. }) => {
                positive(*radius_mm)
            }
            (Self::EdgeDiameter { diameter_mm }, MeasurementQuery::EdgeDiameter { .. }) => {
                positive(*diameter_mm)
            }
            (Self::FaceArea { area_mm2 }, MeasurementQuery::FaceArea { .. }) => positive(*area_mm2),
            (
                Self::PlanarFace {
                    area_mm2,
                    origin_mm,
                    normal,
                },
                MeasurementQuery::PlanarFace { .. },
            ) => {
                positive(*area_mm2)
                    && origin_mm
                        .iter()
                        .all(|v| v.is_finite() && v.abs() <= MAX_COORDINATE_MM)
                    && normal.iter().all(|v| v.is_finite())
                    && (normal.iter().map(|v| v * v).sum::<f64>() - 1.).abs() <= 1e-8
            }
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportedAssetSeal {
    pub asset_id: String,
    pub sha256: String,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MeasurementEngine {
    pub name: String,
    pub protocol_version: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MeasurementReport {
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
    pub query: MeasurementQuery,
    pub result: MeasurementValue,
}

/// Captured by the host from committed bytes, never supplied as authority by UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasurementBinding {
    pub revision_id: String,
    pub body_id: String,
    pub source_sha256: String,
    pub source_size: u64,
    pub document_sha256: String,
    pub imported_asset_seals: Vec<ImportedAssetSeal>,
}

impl MeasurementBinding {
    pub fn valid(&self) -> bool {
        let mut total = 0u64;
        let mut previous: Option<&str> = None;
        uuid::Uuid::parse_str(&self.revision_id).is_ok()
            && self.revision_id.len() == 36
            && !self.body_id.is_empty()
            && self.body_id.len() <= 80
            && self
                .body_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
            && crate::artifacts::valid_digest(&self.source_sha256)
            && crate::artifacts::valid_digest(&self.document_sha256)
            && self.source_size > 0
            && self.source_size <= crate::artifacts::MAX_FILE_BYTES as u64
            && self.imported_asset_seals.len() <= crate::cad_ir::assets::MAX_STEP_ASSETS
            && self.imported_asset_seals.iter().all(|seal| {
                total = total.saturating_add(seal.size);
                let sorted = previous.is_none_or(|id| id < seal.asset_id.as_str());
                previous = Some(&seal.asset_id);
                sorted
                    && crate::cad_ir::assets::valid_reference(&seal.asset_id, &seal.sha256)
                    && seal.size > 0
                    && seal.size <= crate::artifacts::MAX_FILE_BYTES as u64
                    && total <= 128 * 1024 * 1024
            })
    }
}

impl MeasurementReport {
    pub fn matches(
        &self,
        request_id: &str,
        binding: &MeasurementBinding,
        query: &MeasurementQuery,
    ) -> bool {
        binding.valid()
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

#[cfg(test)]
#[path = "schema_tests.rs"]
mod tests;
