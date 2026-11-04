use super::schema::{failure, Request, Response, PROTOCOL_VERSION};

pub(super) fn handle(request: Request) -> Response {
    let Some(document) = request
        .document
        .filter(|document| request.body_id.is_none() && document.is_datum_only())
    else {
        return failure(
            request.request_id,
            "INVALID_REQUEST",
            "Expected a datum-only document",
        );
    };
    if let Err(error) = document.validate() {
        return failure(request.request_id, "DATUM_INVALID", error.to_string());
    }
    if let Err(error) = crate::native::datum::validate(&document) {
        return failure(request.request_id, &error.code, error.detail);
    }
    Response {
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
        measurement_sha256: None,
    }
}
