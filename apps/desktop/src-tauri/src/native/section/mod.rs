//! Exact BREP plane intersection with bounded approximate display curves.
mod client;
#[cfg(feature = "native-occt")]
mod solid;
#[cfg(feature = "native-occt")]
mod source;
use super::NativeError;
pub use client::{section_step, section_with_executable};
use serde::{Deserialize, Serialize};
#[cfg(feature = "native-occt")]
pub use source::execute_source;

pub const MAX_SECTION_BYTES: usize = 8 * 1024 * 1024;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SectionPlane {
    pub origin_mm: [f64; 3],
    pub normal: [f64; 3],
    #[serde(default = "default_deflection")]
    pub deflection_mm: f64,
}
fn default_deflection() -> f64 {
    0.01
}
impl SectionPlane {
    pub fn normalized(&self) -> Result<Self, NativeError> {
        if self
            .origin_mm
            .iter()
            .any(|x| !x.is_finite() || x.abs() > 10000.)
            || self.normal.iter().any(|x| !x.is_finite() || x.abs() > 1e6)
            || !self.deflection_mm.is_finite()
            || !(0.001..=1.).contains(&self.deflection_mm)
        {
            return Err(error(
                "INVALID_SECTION_PLANE",
                "Section plane is nonfinite or out of bounds",
            ));
        }
        let length = self.normal.iter().map(|x| x * x).sum::<f64>().sqrt();
        if length < 1e-9 {
            return Err(error(
                "INVALID_SECTION_PLANE",
                "Section normal is degenerate",
            ));
        }
        Ok(Self {
            origin_mm: self.origin_mm,
            normal: self.normal.map(|x| x / length),
            deflection_mm: self.deflection_mm,
        })
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SectionCurve {
    pub id: String,
    pub length_mm: f64,
    pub points_mm: Vec<[f64; 3]>,
    pub closed: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SectionGeometry {
    pub plane: SectionPlane,
    pub curves: Vec<SectionCurve>,
    pub total_length_mm: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SectionReport {
    pub source_sha256: String,
    pub geometry: SectionGeometry,
}
pub(super) fn error(code: &str, detail: impl Into<String>) -> NativeError {
    NativeError {
        code: code.into(),
        detail: detail.into(),
    }
}
pub(super) fn app_error(error: NativeError) -> crate::core::AppError {
    crate::core::AppError::Invalid(
        serde_json::json!({"code":error.code,"message":error.detail}).to_string(),
    )
}
