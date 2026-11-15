//! Native-independent sealed IGES exchange and measurement contract.
use serde::{Deserialize, Serialize};
pub const IMPORTED_BODY_ID: &str = "imported_body";
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Direction {
    Import,
    Export,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Metrics {
    pub volume_mm3: f64,
    pub area_mm2: f64,
    pub solid_count: i32,
    pub face_count: i32,
    pub edge_count: i32,
    pub bounds_mm: [f64; 3],
}
impl Metrics {
    pub fn valid(&self) -> bool {
        self.volume_mm3.is_finite()
            && self.volume_mm3 > 0.
            && self.area_mm2.is_finite()
            && self.area_mm2 > 0.
            && self.solid_count > 0
            && self.face_count > 0
            && self.edge_count > 0
            && self
                .bounds_mm
                .iter()
                .all(|v| v.is_finite() && *v > 0. && *v <= 1e9)
    }
    pub fn matches(&self, other: &Self) -> bool {
        let close = |a: f64, b: f64| (a - b).abs() <= 1e-4_f64.max(a.abs().max(b.abs()) * 1e-7);
        self.valid()
            && other.valid()
            && self.solid_count == other.solid_count
            && close(self.volume_mm3, other.volume_mm3)
            && close(self.area_mm2, other.area_mm2)
            && self
                .bounds_mm
                .iter()
                .zip(other.bounds_mm)
                .all(|(a, b)| close(*a, b))
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExchangeReport {
    pub schema_version: u32,
    pub request_id: String,
    pub direction: Direction,
    pub source_sha256: String,
    pub source_size: u64,
    pub source: Metrics,
    pub round_trip: Metrics,
    pub output_sha256: String,
    pub output_size: u64,
    pub preview_sha256: Option<String>,
    pub preview_size: Option<u64>,
}
impl ExchangeReport {
    pub fn valid_for(&self, request_id: &str, direction: Direction, hash: &str, size: u64) -> bool {
        let bounded = |v: u64| v > 0 && v <= crate::artifacts::MAX_FILE_BYTES as u64;
        self.schema_version == 1
            && self.request_id == request_id
            && self.direction == direction
            && self.source_sha256 == hash
            && crate::artifacts::valid_digest(hash)
            && self.source_size == size
            && bounded(size)
            && self.source.matches(&self.round_trip)
            && bounded(self.output_size)
            && crate::artifacts::valid_digest(&self.output_sha256)
            && match direction {
                Direction::Import => {
                    self.preview_sha256
                        .as_ref()
                        .is_some_and(|h| crate::artifacts::valid_digest(h))
                        && self.preview_size.is_some_and(bounded)
                }
                Direction::Export => self.preview_sha256.is_none() && self.preview_size.is_none(),
            }
    }
}
