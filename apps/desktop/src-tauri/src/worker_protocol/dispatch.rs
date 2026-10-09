//! Existing build/import/inspect/section dispatch, extracted without geometry changes.
use super::schema::{failure, Request, Response, MAX_STEP_BYTES, PROTOCOL_VERSION};
use crate::native::{inspection, job::execute, step_import};
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

pub(super) fn handle(input: &[u8], cwd: &Path) -> Response {
    let request: Request = match serde_json::from_slice(input) {
        Ok(request) => request,
        Err(error) => return failure(String::new(), "INVALID_REQUEST", error.to_string()),
    };
    if request.protocol_version != PROTOCOL_VERSION || request.request_id.len() > 80 {
        return failure(
            request.request_id,
            "PROTOCOL_MISMATCH",
            "Unsupported CAD worker request",
        );
    }
    if request.operation.as_deref() == Some("measure_pair") {
        return super::pair_measurement::handle(request, cwd);
    }
    if request.pair_measurement_query.is_some() {
        return failure(
            request.request_id,
            "INVALID_REQUEST",
            "Pair measurement query is not accepted by other operations",
        );
    }
    if request.operation.as_deref() == Some("section_reference") {
        return super::face_section::handle(request, cwd);
    }
    if request.face_section_query.is_some() {
        return failure(
            request.request_id,
            "INVALID_REQUEST",
            "Face section query is not accepted by other operations",
        );
    }
    if request.operation.as_deref() == Some("measure_reference") {
        return super::measurement::handle(request, cwd);
    }
    if request.document_sha256.is_some()
        || request.source_size.is_some()
        || request.measurement_query.is_some()
    {
        return failure(
            request.request_id,
            "INVALID_REQUEST",
            "Measurement fields are not accepted by other worker operations",
        );
    }
    if request.operation.as_deref() == Some("section_step") {
        if request.document.is_some() || request.body_id.is_some() {
            return failure(
                request.request_id,
                "INVALID_REQUEST",
                "Section accepts only a staged STEP source",
            );
        }
        let (Some(hash), Some(plane)) = (
            request.source_sha256.as_deref(),
            request.section_plane.as_ref(),
        ) else {
            return failure(
                request.request_id,
                "INVALID_REQUEST",
                "Section source checksum and plane are required",
            );
        };
        if let Err(error) = crate::native::section::execute_source(cwd, hash, plane) {
            return failure(request.request_id, &error.code, error.detail);
        }
        let bytes = match fs::read(cwd.join("section.json")) {
            Ok(bytes) => bytes,
            Err(error) => return failure(request.request_id, "SECTION_FAILED", error.to_string()),
        };
        return Response {
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
            section_sha256: Some(format!("{:x}", Sha256::digest(&bytes))),
            measurement_sha256: None,
        };
    }
    if request.section_plane.is_some() || request.source_sha256.is_some() {
        return failure(
            request.request_id,
            "INVALID_REQUEST",
            "Section fields are not accepted for other worker operations",
        );
    }
    let inspection_only = request.operation.as_deref() == Some("inspect_step");
    let result = match request.operation.as_deref() {
        Some("import_step") if request.document.is_none() && request.body_id.is_none() => {
            step_import::execute(cwd)
        }
        Some("inspect_step") if request.document.is_none() && request.body_id.is_none() => {
            inspection::inspect_source(cwd)
        }
        None => match request.document.as_ref() {
            Some(document) => execute(document, request.body_id.as_deref(), cwd),
            None => {
                return failure(
                    request.request_id,
                    "INVALID_REQUEST",
                    "CAD document is missing",
                )
            }
        },
        _ => {
            return failure(
                request.request_id,
                "INVALID_REQUEST",
                "Unsupported CAD worker operation",
            )
        }
    };
    let result = match result {
        Ok(result) => result,
        Err(error) => return failure(request.request_id, &error.code, error.detail),
    };
    if inspection_only {
        return Response {
            protocol_version: PROTOCOL_VERSION,
            request_id: request.request_id,
            status: "completed",
            code: None,
            detail: None,
            volume_mm3: Some(result.volume_mm3),
            area_mm2: Some(result.area_mm2),
            face_count: Some(result.face_count),
            edge_count: Some(result.edge_count),
            bounds_mm: Some(result.bounds_mm),
            step_sha256: None,
            preview_sha256: None,
            section_sha256: None,
            measurement_sha256: None,
        };
    }
    let path = cwd.join("model.step");
    let bytes = match fs::read(&path) {
        Ok(bytes) if !bytes.is_empty() && bytes.len() as u64 <= MAX_STEP_BYTES => bytes,
        Ok(_) => {
            return failure(
                request.request_id,
                "STEP_SIZE_INVALID",
                "STEP output exceeded the limit",
            )
        }
        Err(error) => return failure(request.request_id, "STEP_READ_FAILED", error.to_string()),
    };
    let preview_path = cwd.join("preview.glb");
    let preview = match fs::read(&preview_path) {
        Ok(bytes) if !bytes.is_empty() && bytes.len() as u64 <= MAX_STEP_BYTES => bytes,
        Ok(_) => {
            return failure(
                request.request_id,
                "PREVIEW_SIZE_INVALID",
                "Preview output exceeded the limit",
            )
        }
        Err(error) => return failure(request.request_id, "PREVIEW_READ_FAILED", error.to_string()),
    };
    Response {
        protocol_version: PROTOCOL_VERSION,
        request_id: request.request_id,
        status: "completed",
        code: None,
        detail: None,
        volume_mm3: Some(result.volume_mm3),
        area_mm2: Some(result.area_mm2),
        face_count: Some(result.face_count),
        edge_count: Some(result.edge_count),
        bounds_mm: Some(result.bounds_mm),
        step_sha256: Some(format!("{:x}", Sha256::digest(&bytes))),
        preview_sha256: Some(format!("{:x}", Sha256::digest(&preview))),
        section_sha256: None,
        measurement_sha256: None,
    }
}
