//! Worker-only snap execution against sealed staged inputs. No live-project paths or writes.
use super::solid::error;
use crate::{
    cad_ir::Document,
    model_snap::schema::{
        SnapBinding, SnapEngine, SnapQuery, SnapReport, MAX_DOCUMENT_BYTES, MAX_REPORT_BYTES,
    },
    native::{
        build_body_with_assets,
        reference_measurements::inputs::{checked_bytes, imported_seals},
        NativeError,
    },
};
use std::path::Path;

pub fn execute(
    cwd: &Path,
    request_id: &str,
    body_id: &str,
    document_sha256: &str,
    source_sha256: &str,
    source_size: u64,
    query: &SnapQuery,
) -> Result<String, NativeError> {
    if uuid::Uuid::parse_str(request_id).is_err()
        || request_id.len() != 36
        || !query.valid()
        || !crate::artifacts::valid_digest(document_sha256)
        || !crate::artifacts::valid_digest(source_sha256)
        || source_size == 0
        || source_size > crate::artifacts::MAX_FILE_BYTES as u64
    {
        return Err(error(
            "INVALID_SNAP_QUERY",
            "Invalid snap identity, source binding or query",
        ));
    }
    let raw = checked_bytes(cwd, Path::new("document.json"), MAX_DOCUMENT_BYTES)?;
    let source = checked_bytes(
        cwd,
        Path::new("source.step"),
        crate::artifacts::MAX_FILE_BYTES,
    )?;
    if crate::artifacts::digest(&raw) != document_sha256
        || crate::artifacts::digest(&source) != source_sha256
        || source.len() as u64 != source_size
    {
        return Err(error(
            "ASSET_CHECKSUM_MISMATCH",
            "Staged snap source differs from its seal",
        ));
    }
    let document: Document =
        serde_json::from_slice(&raw).map_err(|e| error("CAD_IR_INVALID", e.to_string()))?;
    document
        .validate()
        .map_err(|e| error("CAD_IR_INVALID", e.to_string()))?;
    let seals = imported_seals(&document, cwd)?;
    let binding = SnapBinding {
        revision_id: document.revision_id.clone(),
        body_id: body_id.into(),
        source_sha256: source_sha256.into(),
        source_size,
        document_sha256: document_sha256.into(),
        imported_asset_seals: seals.clone(),
    };
    if !binding.valid() || !document.bodies.iter().any(|body| body.id == body_id) {
        return Err(error(
            "INVALID_SNAP_QUERY",
            "Snap body or document revision is invalid",
        ));
    }
    let solid = build_body_with_assets(&document, body_id, Some(cwd))?;
    let value = solid.snap_reference(query)?;
    if checked_bytes(cwd, Path::new("document.json"), MAX_DOCUMENT_BYTES)? != raw
        || checked_bytes(
            cwd,
            Path::new("source.step"),
            crate::artifacts::MAX_FILE_BYTES,
        )? != source
        || imported_seals(&document, cwd)? != seals
    {
        return Err(error(
            "SNAP_STALE",
            "Staged snap inputs changed during computation",
        ));
    }
    let report = SnapReport {
        schema_version: 1,
        request_id: request_id.into(),
        engine: SnapEngine {
            name: "occt".into(),
            protocol_version: 1,
        },
        evaluation_source: "rebuiltAuthoredBody".into(),
        revision_id: document.revision_id,
        body_id: body_id.into(),
        source_sha256: source_sha256.into(),
        source_size,
        document_sha256: document_sha256.into(),
        imported_asset_seals: seals,
        query: query.clone(),
        result: value,
    };
    if !report.matches(request_id, &binding, query) {
        return Err(error(
            "SNAP_FAILED",
            "Native snap report does not match the captured query",
        ));
    }
    let bytes = serde_json::to_vec(&report).map_err(|e| error("SNAP_FAILED", e.to_string()))?;
    if bytes.len() > MAX_REPORT_BYTES {
        return Err(error("SNAP_FAILED", "Native snap report exceeds bounds"));
    }
    let temporary = crate::security::guarded(cwd, Path::new("snap.json.tmp"))
        .map_err(|e| error("ASSET_INVALID", e.to_string()))?;
    let target = crate::security::guarded(cwd, Path::new("snap.json"))
        .map_err(|e| error("ASSET_INVALID", e.to_string()))?;
    std::fs::write(&temporary, &bytes)
        .and_then(|_| std::fs::rename(temporary, target))
        .map_err(|e| error("SNAP_FAILED", e.to_string()))?;
    Ok(crate::artifacts::digest(&bytes))
}
