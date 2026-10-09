use crate::{
    cad_ir::Document,
    model_face_section::schema::{
        FaceSectionQuery, FaceSectionReport, ResolvedFace, MAX_FACE_SECTION_BYTES,
    },
    model_measurement::schema::{
        MeasurementBinding, MeasurementEngine, MeasurementQuery, MeasurementValue,
        MAX_DOCUMENT_BYTES,
    },
    native::{
        build_body_with_assets,
        reference_measurements::execute::{checked_bytes, imported_seals},
        NativeError, Solid,
    },
};
use std::path::Path;

fn failure(code: &str, detail: impl Into<String>) -> NativeError {
    NativeError {
        code: code.into(),
        detail: detail.into(),
    }
}

pub fn execute(
    cwd: &Path,
    request_id: &str,
    body_id: &str,
    document_hash: &str,
    source_hash: &str,
    source_size: u64,
    query: &FaceSectionQuery,
) -> Result<String, NativeError> {
    if uuid::Uuid::parse_str(request_id).is_err() || request_id.len() != 36 || !query.valid() {
        return Err(failure(
            "INVALID_SECTION_PLANE",
            "Face section identity or query is invalid",
        ));
    }
    let raw = checked_bytes(cwd, Path::new("document.json"), MAX_DOCUMENT_BYTES)?;
    let source = checked_bytes(
        cwd,
        Path::new("source.step"),
        crate::artifacts::MAX_FILE_BYTES,
    )?;
    if crate::artifacts::digest(&raw) != document_hash
        || crate::artifacts::digest(&source) != source_hash
        || source.len() as u64 != source_size
    {
        return Err(failure(
            "ASSET_CHECKSUM_MISMATCH",
            "Section inputs differ from captured seals",
        ));
    }
    let document: Document =
        serde_json::from_slice(&raw).map_err(|e| failure("CAD_IR_INVALID", e.to_string()))?;
    document
        .validate()
        .map_err(|e| failure("CAD_IR_INVALID", e.to_string()))?;
    let seals = imported_seals(&document, cwd)?;
    let binding = MeasurementBinding {
        revision_id: document.revision_id.clone(),
        body_id: body_id.into(),
        source_sha256: source_hash.into(),
        source_size,
        document_sha256: document_hash.into(),
        imported_asset_seals: seals,
    };
    if !binding.valid() {
        return Err(failure(
            "INVALID_SECTION_PLANE",
            "Face section binding is invalid",
        ));
    }
    let selected = build_body_with_assets(&document, body_id, Some(cwd))?;
    let planar = selected.measure_reference(&MeasurementQuery::PlanarFace {
        reference: query.reference.clone(),
    })?;
    let MeasurementValue::PlanarFace {
        area_mm2,
        origin_mm,
        normal,
    } = planar
    else {
        return Err(failure(
            "SECTION_FAILED",
            "Kernel returned another face measurement kind",
        ));
    };
    let face = ResolvedFace {
        area_mm2,
        origin_mm,
        normal,
    };
    let plane = face.plane(query);
    if !face.valid_for(query) || !plane.valid() {
        return Err(failure(
            "INVALID_SECTION_PLANE",
            "Resolved face section origin exceeds native section bounds",
        ));
    }
    // Scope is ALL committed STEP geometry, including viewer-hidden bodies.
    // Never substitute the selected rebuilt body for the sealed whole source.
    let source_path = crate::security::guarded(cwd, Path::new("source.step"))
        .map_err(|e| failure("ASSET_INVALID", e.to_string()))?;
    let whole = Solid::read_step(&source_path)?;
    let geometry = whole
        .section(&crate::native::section::SectionPlane {
            origin_mm: plane.origin_mm,
            normal: plane.normal,
            deflection_mm: plane.deflection_mm,
        })?
        .into();
    let report = FaceSectionReport {
        schema_version: 1,
        request_id: request_id.into(),
        engine: MeasurementEngine {
            name: "occt".into(),
            protocol_version: 1,
        },
        evaluation_source: "rebuiltAuthoredFaceAndSealedStep".into(),
        revision_id: binding.revision_id.clone(),
        body_id: binding.body_id.clone(),
        source_sha256: binding.source_sha256.clone(),
        source_size: binding.source_size,
        document_sha256: binding.document_sha256.clone(),
        imported_asset_seals: binding.imported_asset_seals.clone(),
        query: query.clone(),
        face,
        geometry,
    };
    if !report.matches(request_id, &binding, query) {
        return Err(failure(
            "SECTION_FAILED",
            "Face section report is outside bounded geometry contract",
        ));
    }
    if checked_bytes(cwd, Path::new("document.json"), MAX_DOCUMENT_BYTES)? != raw
        || checked_bytes(
            cwd,
            Path::new("source.step"),
            crate::artifacts::MAX_FILE_BYTES,
        )? != source
        || imported_seals(&document, cwd)? != binding.imported_asset_seals
    {
        return Err(failure(
            "SECTION_STALE",
            "Section inputs changed during execution",
        ));
    }
    let bytes =
        serde_json::to_vec(&report).map_err(|e| failure("SECTION_FAILED", e.to_string()))?;
    if bytes.len() > MAX_FACE_SECTION_BYTES {
        return Err(failure(
            "SECTION_LIMIT",
            "Face section report exceeds 8 MiB",
        ));
    }
    crate::artifacts::immutable_write(cwd, Path::new("face-section.json"), &bytes)
        .map_err(|e| failure("SECTION_FAILED", e.to_string()))?;
    Ok(crate::artifacts::digest(&bytes))
}
