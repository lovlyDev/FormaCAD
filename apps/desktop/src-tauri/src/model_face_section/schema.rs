//! Renderer supplies face identity and signed offset only; no plane authority.
use crate::{
    cad_ir::{TopologyKind, TopologyReference},
    model_measurement::schema::{
        ImportedAssetSeal, MeasurementBinding, MeasurementEngine, MeasurementQuery,
        MeasurementValue,
    },
};
use serde::{Deserialize, Serialize};

pub const MAX_FACE_SECTION_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_WORLD_ORIGIN_MM: f64 = 10_000.;
const MAX_CURVES: usize = 4096;
const MAX_POINTS: usize = 100_000;
const MAX_POINT_COORDINATE_MM: f64 = 1_000_000_000.;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FaceSectionQuery {
    pub reference: TopologyReference,
    pub offset_mm: f64,
    pub deflection_mm: f64,
}
impl FaceSectionQuery {
    pub fn valid(&self) -> bool {
        self.reference.is_valid()
            && self.reference.kind == TopologyKind::Face
            && self.offset_mm.is_finite()
            && self.offset_mm.abs() <= MAX_WORLD_ORIGIN_MM
            && self.deflection_mm.is_finite()
            && (0.001..=1.).contains(&self.deflection_mm)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResolvedFace {
    pub area_mm2: f64,
    pub origin_mm: [f64; 3],
    pub normal: [f64; 3],
}
impl ResolvedFace {
    pub fn valid_for(&self, query: &FaceSectionQuery) -> bool {
        MeasurementValue::PlanarFace {
            area_mm2: self.area_mm2,
            origin_mm: self.origin_mm,
            normal: self.normal,
        }
        .valid_for(&MeasurementQuery::PlanarFace {
            reference: query.reference.clone(),
        })
    }
    pub fn plane(&self, query: &FaceSectionQuery) -> ResolvedPlane {
        ResolvedPlane {
            origin_mm: std::array::from_fn(|axis| {
                self.origin_mm[axis] + query.offset_mm * self.normal[axis]
            }),
            normal: self.normal,
            deflection_mm: query.deflection_mm,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResolvedPlane {
    pub origin_mm: [f64; 3],
    pub normal: [f64; 3],
    pub deflection_mm: f64,
}
impl ResolvedPlane {
    pub fn valid(&self) -> bool {
        self.origin_mm
            .iter()
            .all(|v| v.is_finite() && v.abs() <= MAX_WORLD_ORIGIN_MM)
            && self.normal.iter().all(|v| v.is_finite())
            && (self.normal.iter().map(|v| v * v).sum::<f64>() - 1.).abs() <= 1e-8
            && self.deflection_mm.is_finite()
            && (0.001..=1.).contains(&self.deflection_mm)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FaceSectionCurve {
    pub id: String,
    pub length_mm: f64,
    pub points_mm: Vec<[f64; 3]>,
    pub closed: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FaceSectionGeometry {
    pub plane: ResolvedPlane,
    pub curves: Vec<FaceSectionCurve>,
    pub total_length_mm: f64,
}
fn near(actual: f64, expected: f64) -> bool {
    actual.is_finite()
        && expected.is_finite()
        && (actual - expected).abs() <= 1e-9 * actual.abs().max(expected.abs()).max(1.)
}
impl FaceSectionGeometry {
    pub fn valid_for(&self, face: &ResolvedFace, query: &FaceSectionQuery) -> bool {
        let expected = face.plane(query);
        if !self.plane.valid()
            || !expected.valid()
            || self.curves.len() > MAX_CURVES
            || !near(self.plane.deflection_mm, expected.deflection_mm)
            || !self
                .plane
                .origin_mm
                .iter()
                .zip(expected.origin_mm)
                .all(|(a, b)| near(*a, b))
            || !self
                .plane
                .normal
                .iter()
                .zip(expected.normal)
                .all(|(a, b)| near(*a, b))
            || !self.total_length_mm.is_finite()
            || self.total_length_mm < 0.
        {
            return false;
        }
        let mut point_count = 0usize;
        let mut total = 0.;
        for (index, curve) in self.curves.iter().enumerate() {
            point_count = point_count.saturating_add(curve.points_mm.len());
            if curve.id != format!("section-edge-{}", index + 1)
                || curve.points_mm.len() < 2
                || point_count > MAX_POINTS
                || !curve.length_mm.is_finite()
                || curve.length_mm <= 0.
            {
                return false;
            }
            for point in &curve.points_mm {
                if !point
                    .iter()
                    .all(|v| v.is_finite() && v.abs() <= MAX_POINT_COORDINATE_MM)
                {
                    return false;
                }
                let signed = point
                    .iter()
                    .zip(self.plane.origin_mm)
                    .zip(self.plane.normal)
                    .map(|((p, o), n)| (p - o) * n)
                    .sum::<f64>();
                if !signed.is_finite() || signed.abs() > 1e-6 {
                    return false;
                }
            }
            let first = curve.points_mm.first().unwrap();
            let last = curve.points_mm.last().unwrap();
            let closed = first
                .iter()
                .zip(last)
                .map(|(a, b)| (a - b).powi(2))
                .sum::<f64>()
                <= 1e-12;
            if closed != curve.closed {
                return false;
            }
            total += curve.length_mm;
        }
        near(total, self.total_length_mm)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FaceSectionReport {
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
    pub query: FaceSectionQuery,
    pub face: ResolvedFace,
    pub geometry: FaceSectionGeometry,
}
impl FaceSectionReport {
    pub fn matches(
        &self,
        id: &str,
        binding: &MeasurementBinding,
        query: &FaceSectionQuery,
    ) -> bool {
        binding.valid()
            && query.valid()
            && self.schema_version == 1
            && uuid::Uuid::parse_str(id).is_ok()
            && id.len() == 36
            && self.request_id == id
            && self.engine.name == "occt"
            && self.engine.protocol_version == 1
            && self.evaluation_source == "rebuiltAuthoredFaceAndSealedStep"
            && self.revision_id == binding.revision_id
            && self.body_id == binding.body_id
            && self.source_sha256 == binding.source_sha256
            && self.source_size == binding.source_size
            && self.document_sha256 == binding.document_sha256
            && self.imported_asset_seals == binding.imported_asset_seals
            && self.query == *query
            && self.face.valid_for(query)
            && self.geometry.valid_for(&self.face, query)
    }
}

#[cfg(feature = "native-occt")]
impl From<crate::native::section::SectionGeometry> for FaceSectionGeometry {
    fn from(value: crate::native::section::SectionGeometry) -> Self {
        Self {
            plane: ResolvedPlane {
                origin_mm: value.plane.origin_mm,
                normal: value.plane.normal,
                deflection_mm: value.plane.deflection_mm,
            },
            total_length_mm: value.total_length_mm,
            curves: value
                .curves
                .into_iter()
                .map(|curve| FaceSectionCurve {
                    id: curve.id,
                    length_mm: curve.length_mm,
                    points_mm: curve.points_mm,
                    closed: curve.closed,
                })
                .collect(),
        }
    }
}

#[cfg(test)]
#[path = "schema_tests.rs"]
mod tests;
