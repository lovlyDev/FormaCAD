use super::schema::{failure, Request, Response, PROTOCOL_VERSION};
use crate::iges::schema::Direction;
pub(super) fn handle(request: Request, cwd: &std::path::Path) -> Response {
    let direction = if request.operation.as_deref() == Some("iges_import") {
        Direction::Import
    } else {
        Direction::Export
    };
    if request.document.is_some()
        || request.body_id.is_some()
        || request.document_sha256.is_some()
        || request.section_plane.is_some()
        || request.measurement_query.is_some()
        || request.face_section_query.is_some()
        || request.pair_measurement_query.is_some()
        || request.snap_query.is_some()
        || uuid::Uuid::parse_str(&request.request_id).is_err()
    {
        return failure(
            request.request_id,
            "INVALID_REQUEST",
            "IGES accepts only a fixed staged source and its seal",
        );
    }
    let (Some(hash), Some(size)) = (request.source_sha256.as_deref(), request.source_size) else {
        return failure(
            request.request_id,
            "INVALID_REQUEST",
            "IGES input seal is required",
        );
    };
    if !crate::artifacts::valid_digest(hash)
        || size == 0
        || size > crate::artifacts::MAX_FILE_BYTES as u64
    {
        return failure(
            request.request_id,
            "INVALID_REQUEST",
            "Invalid IGES input seal",
        );
    }
    match crate::native::iges::execute(cwd, &request.request_id, direction, hash, size) {
        Err(error) => failure(request.request_id, &error.code, error.detail),
        Ok(report) => {
            let bytes = match std::fs::read(cwd.join("iges-report.json")) {
                Ok(v) => v,
                Err(e) => return failure(request.request_id, "IGES_REPORT_INVALID", e.to_string()),
            };
            Response {
                protocol_version: PROTOCOL_VERSION,
                request_id: request.request_id,
                status: "completed",
                code: None,
                detail: None,
                volume_mm3: Some(report.round_trip.volume_mm3),
                area_mm2: Some(report.round_trip.area_mm2),
                face_count: Some(report.round_trip.face_count),
                edge_count: Some(report.round_trip.edge_count),
                bounds_mm: Some(report.round_trip.bounds_mm),
                step_sha256: if direction == Direction::Import {
                    Some(report.output_sha256)
                } else {
                    None
                },
                preview_sha256: report.preview_sha256,
                section_sha256: None,
                measurement_sha256: Some(crate::artifacts::digest(&bytes)),
            }
        }
    }
}
