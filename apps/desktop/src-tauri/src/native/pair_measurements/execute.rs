//! Worker-side reconstruction. Fixed staged names; no STEP/GLB output.
fn error(code: &str, detail: impl Into<String>) -> crate::native::NativeError {
    crate::native::NativeError {
        code: code.into(),
        detail: detail.into(),
    }
}
use crate::model_pair_measurement::schema::{
    PairMeasurementQuery, PairMeasurementReport, MAX_PAIR_REPORT_BYTES,
};
use crate::{
    cad_ir::Document,
    model_measurement::schema::{MeasurementEngine, MAX_DOCUMENT_BYTES},
    native::{build_body_with_assets, NativeError},
};

use std::path::Path;

pub use crate::native::reference_measurements::execute::{checked_bytes, imported_seals};
fn digest(bytes: &[u8]) -> String {
    crate::artifacts::digest(bytes)
}
pub fn execute(
    cwd: &Path,
    request_id: &str,
    body_id: &str,
    document_sha256: &str,
    source_sha256: &str,
    source_size: u64,
    query: &PairMeasurementQuery,
) -> Result<String, NativeError> {
    if uuid::Uuid::parse_str(request_id).is_err()
        || request_id.len() != 36
        || body_id.is_empty()
        || body_id.len() > 80
        || !body_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        || !crate::artifacts::valid_digest(document_sha256)
        || !crate::artifacts::valid_digest(source_sha256)
        || source_size == 0
        || source_size > crate::artifacts::MAX_FILE_BYTES as u64
        || !query.valid()
    {
        return Err(error(
            "INVALID_MEASUREMENT_QUERY",
            "Invalid request identity, seal or reference",
        ));
    }
    let raw = checked_bytes(cwd, Path::new("document.json"), MAX_DOCUMENT_BYTES)?;
    if digest(&raw) != document_sha256 {
        return Err(error(
            "ASSET_CHECKSUM_MISMATCH",
            "Raw CAD document checksum mismatch",
        ));
    }
    let source = checked_bytes(
        cwd,
        Path::new("source.step"),
        crate::artifacts::MAX_FILE_BYTES,
    )?;
    if source.len() as u64 != source_size || digest(&source) != source_sha256 {
        return Err(error(
            "ASSET_CHECKSUM_MISMATCH",
            "Committed STEP checksum/size mismatch",
        ));
    }
    let document: Document =
        serde_json::from_slice(&raw).map_err(|e| error("CAD_IR_INVALID", e.to_string()))?;
    document
        .validate()
        .map_err(|e| error("CAD_IR_INVALID", e.to_string()))?;
    if !document.bodies.iter().any(|body| body.id == body_id) {
        return Err(error("MISSING_BODY", "Named measurement body is missing"));
    }
    let imported_asset_seals = imported_seals(&document, cwd)?;
    let solid = build_body_with_assets(&document, body_id, Some(cwd))?;
    let value = solid.measure_reference_pair(query)?;
    // Recheck the staged input identity, independently of live-project host rechecks.
    if checked_bytes(cwd, Path::new("document.json"), MAX_DOCUMENT_BYTES)? != raw
        || checked_bytes(
            cwd,
            Path::new("source.step"),
            crate::artifacts::MAX_FILE_BYTES,
        )? != source
        || imported_seals(&document, cwd)? != imported_asset_seals
    {
        return Err(error(
            "MEASUREMENT_STALE",
            "Staged measurement inputs changed during execution",
        ));
    }
    let report = PairMeasurementReport {
        schema_version: 1,
        request_id: request_id.into(),
        engine: MeasurementEngine {
            name: "occt".into(),
            protocol_version: 1,
        },
        evaluation_source: "rebuiltAuthoredBody".into(),
        revision_id: document.revision_id,
        body_id: body_id.into(),
        source_sha256: source_sha256.into(),
        source_size,
        document_sha256: document_sha256.into(),
        imported_asset_seals,
        query: query.clone(),
        result: value,
    };
    let bytes = serde_json::to_vec(&report)
        .map_err(|e| error("MEASUREMENT_INVALID_RESULT", e.to_string()))?;
    if bytes.len() > MAX_PAIR_REPORT_BYTES {
        return Err(error(
            "MEASUREMENT_INVALID_RESULT",
            "Measurement report exceeds limit",
        ));
    }
    let temporary = crate::security::guarded(cwd, Path::new("pair-measurement.json.tmp"))
        .map_err(|e| error("ASSET_INVALID", e.to_string()))?;
    let target = crate::security::guarded(cwd, Path::new("pair-measurement.json"))
        .map_err(|e| error("ASSET_INVALID", e.to_string()))?;
    std::fs::write(&temporary, &bytes)
        .map_err(|e| error("MEASUREMENT_INVALID_RESULT", e.to_string()))?;
    std::fs::rename(temporary, target)
        .map_err(|e| error("MEASUREMENT_INVALID_RESULT", e.to_string()))?;
    Ok(digest(&bytes))
}
