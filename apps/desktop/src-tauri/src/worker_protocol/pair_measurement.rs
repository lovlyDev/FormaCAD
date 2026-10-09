use super::schema::{failure, Request, Response, PROTOCOL_VERSION};
use std::path::Path;
pub(super) fn handle(request: Request, cwd: &Path) -> Response {
    if request.document.is_some()
        || request.section_plane.is_some()
        || request.measurement_query.is_some()
        || request.face_section_query.is_some()
    {
        return failure(
            request.request_id,
            "INVALID_REQUEST",
            "Pair measurement reads only fixed staged inputs and two exact references",
        );
    }
    let (Some(body), Some(document_hash), Some(source_hash), Some(source_size), Some(query)) = (
        request.body_id.as_deref(),
        request.document_sha256.as_deref(),
        request.source_sha256.as_deref(),
        request.source_size,
        request.pair_measurement_query.as_ref(),
    ) else {
        return failure(
            request.request_id,
            "INVALID_REQUEST",
            "Pair measurement identity, sealed inputs and query are required",
        );
    };
    match crate::native::pair_measurements::execute::execute(
        cwd,
        &request.request_id,
        body,
        document_hash,
        source_hash,
        source_size,
        query,
    ) {
        Ok(hash) => Response {
            protocol_version: PROTOCOL_VERSION,
            request_id: request.request_id,
            status: "completed",
            code: None,
            detail: None,
            volume_mm3: None,
            area_mm2: None,
            face_count: None,
            edge_count: None,
            bounds_mm: None,
            step_sha256: None,
            preview_sha256: None,
            section_sha256: None,
            measurement_sha256: Some(hash),
        },
        Err(error) => failure(request.request_id, &error.code, error.detail),
    }
}
