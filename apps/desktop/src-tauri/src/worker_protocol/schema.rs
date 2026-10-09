use crate::model_face_section::schema::FaceSectionQuery;
use crate::model_pair_measurement::schema::PairMeasurementQuery;
use crate::{
    cad_ir::Document, model_measurement::schema::MeasurementQuery, native::section::SectionPlane,
};
use serde::{Deserialize, Serialize};
pub(super) const PROTOCOL_VERSION: u32 = 1;
pub(super) const MAX_REQUEST_BYTES: u64 = 4 * 1024 * 1024;
pub(super) const MAX_STEP_BYTES: u64 = 40 * 1024 * 1024;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Request {
    pub protocol_version: u32,
    pub request_id: String,
    pub body_id: Option<String>,
    pub operation: Option<String>,
    pub document: Option<Document>,
    pub source_sha256: Option<String>,
    pub section_plane: Option<SectionPlane>,
    pub document_sha256: Option<String>,
    pub source_size: Option<u64>,
    pub measurement_query: Option<MeasurementQuery>,
    pub face_section_query: Option<FaceSectionQuery>,
    pub pair_measurement_query: Option<PairMeasurementQuery>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Response {
    pub protocol_version: u32,
    pub request_id: String,
    pub status: &'static str,
    pub code: Option<String>,
    pub detail: Option<String>,
    pub volume_mm3: Option<f64>,
    pub area_mm2: Option<f64>,
    pub face_count: Option<i32>,
    pub edge_count: Option<i32>,
    pub bounds_mm: Option<[f64; 3]>,
    pub step_sha256: Option<String>,
    pub preview_sha256: Option<String>,
    pub section_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub measurement_sha256: Option<String>,
}

pub(super) fn failure(request_id: String, code: &str, detail: impl Into<String>) -> Response {
    Response {
        protocol_version: PROTOCOL_VERSION,
        request_id,
        status: "error",
        code: Some(code.into()),
        detail: Some(detail.into()),
        volume_mm3: None,
        area_mm2: None,
        face_count: None,
        edge_count: None,
        bounds_mm: None,
        step_sha256: None,
        preview_sha256: None,
        section_sha256: None,
        measurement_sha256: None,
    }
}
