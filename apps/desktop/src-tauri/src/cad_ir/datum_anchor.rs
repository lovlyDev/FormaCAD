//! Stable source and relative placement parameters for construction geometry.
#[cfg(feature = "native-occt")]
use super::Operation;
use super::{TopologyKind, TopologyReference, MAX_DIMENSION_MM};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum DatumSource {
    Topology {
        body_id: String,
        reference: TopologyReference,
    },
    Plane {
        feature_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum DatumGeometry {
    Plane { side_mm: f64 },
    Axis { length_mm: f64 },
    Point,
}

impl DatumGeometry {
    pub(super) fn valid(&self) -> bool {
        match *self {
            Self::Plane { side_mm: span } | Self::Axis { length_mm: span } => {
                span.is_finite() && span > 1e-7 && span <= MAX_DIMENSION_MM
            }
            Self::Point => true,
        }
    }

    #[cfg(feature = "native-occt")]
    pub(crate) fn placed(&self, origin_mm: [f64; 3], direction: [f64; 3]) -> Operation {
        match *self {
            Self::Plane { side_mm } => Operation::DatumPlane {
                origin_mm,
                normal: direction,
                side_mm,
            },
            Self::Axis { length_mm } => Operation::DatumAxis {
                origin_mm,
                direction,
                length_mm,
            },
            Self::Point => Operation::DatumPoint { origin_mm },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DatumAnchor {
    pub source: DatumSource,
    pub offset_mm: f64,
    pub angle_deg: f64,
}

impl DatumAnchor {
    pub(super) fn valid(&self) -> bool {
        self.offset_mm.is_finite()
            && self.offset_mm.abs() <= MAX_DIMENSION_MM
            && self.angle_deg.is_finite()
            && self.angle_deg.abs() <= 360.
            && match &self.source {
                DatumSource::Topology { body_id, reference } => {
                    super::validation::valid_id(body_id)
                        && reference.is_valid()
                        && matches!(reference.kind, TopologyKind::Face | TopologyKind::Edge)
                }
                DatumSource::Plane { feature_id } => super::validation::valid_id(feature_id),
            }
    }

    /// Offset is along the unrotated source direction. The direction tilts
    /// toward a deterministic tangent in the source plane.
    #[cfg(feature = "native-occt")]
    pub(crate) fn place(
        &self,
        origin: [f64; 3],
        direction: [f64; 3],
    ) -> Option<([f64; 3], [f64; 3])> {
        let norm = |v: [f64; 3]| v[0].hypot(v[1]).hypot(v[2]);
        let length = norm(direction);
        if !length.is_finite() || length <= 1e-7 {
            return None;
        }
        let unit = direction.map(|component| component / length);
        let axis = if unit[0].abs() < 0.9 { 0 } else { 1 };
        let tangent =
            std::array::from_fn(|i| if i == axis { 1. } else { 0. } - unit[axis] * unit[i]);
        let tangent_length = norm(tangent);
        let (sin, cos) = self.angle_deg.to_radians().sin_cos();
        let shifted = std::array::from_fn(|i| origin[i] + unit[i] * self.offset_mm);
        let tilted = std::array::from_fn(|i| unit[i] * cos + tangent[i] / tangent_length * sin);
        Some((shifted, tilted))
    }
}
