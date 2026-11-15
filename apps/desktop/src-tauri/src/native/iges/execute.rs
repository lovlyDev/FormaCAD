use crate::{
    iges::schema::{Direction, ExchangeReport, Metrics},
    native::{NativeError, Solid},
};
use std::{fs, path::Path};
fn error(code: &str, detail: impl Into<String>) -> NativeError {
    NativeError {
        code: code.into(),
        detail: detail.into(),
    }
}
fn bytes(path: &Path) -> Result<Vec<u8>, NativeError> {
    let meta = fs::symlink_metadata(path).map_err(|e| error("IGES_IO_FAILED", e.to_string()))?;
    if !meta.is_file() || meta.len() == 0 || meta.len() > crate::artifacts::MAX_FILE_BYTES as u64 {
        return Err(error(
            "IGES_SIZE_INVALID",
            "Exchange input/output size is invalid",
        ));
    }
    fs::read(path).map_err(|e| error("IGES_IO_FAILED", e.to_string()))
}
fn metrics(solid: &Solid) -> Metrics {
    Metrics {
        volume_mm3: solid.volume_mm3(),
        area_mm2: solid.surface_area_mm2(),
        solid_count: solid.solid_count(),
        face_count: solid.face_count(),
        edge_count: solid.edge_count(),
        bounds_mm: solid.bounds_mm(),
    }
}
pub fn execute(
    cwd: &Path,
    request_id: &str,
    direction: Direction,
    hash: &str,
    size: u64,
) -> Result<ExchangeReport, NativeError> {
    let importing = direction == Direction::Import;
    let input = cwd.join(if importing {
        "source.igs"
    } else {
        "source.step"
    });
    let original = bytes(&input)?;
    if original.len() as u64 != size || crate::artifacts::digest(&original) != hash {
        return Err(error(
            "ASSET_CHECKSUM_MISMATCH",
            "IGES exchange source seal mismatch",
        ));
    }
    let source = if importing {
        Solid::read_iges(&input)?
    } else {
        Solid::read_step(&input)?
    };
    let path = cwd.join(if importing { "model.step" } else { "model.igs" });
    let restored = if importing {
        source.write_step(&path)?;
        Solid::read_step(&path)?
    } else {
        source.write_iges(&path)?;
        Solid::read_iges(&path)?
    };
    let output = bytes(&path)?;
    let preview = if importing {
        let body_path = cwd.join("iges-body.glb");
        restored.write_glb(&body_path)?;
        let id = crate::iges::schema::IMPORTED_BODY_ID;
        crate::native::glb_merge::merge_body_glbs(
            &[crate::native::glb_merge::BodyGlb {
                id,
                name: id,
                path: &body_path,
            }],
            &serde_json::json!([]),
            &cwd.join("preview.glb"),
        )?;
        Some(bytes(&cwd.join("preview.glb"))?)
    } else {
        None
    };
    let report = ExchangeReport {
        schema_version: 1,
        request_id: request_id.into(),
        direction,
        source_sha256: hash.into(),
        source_size: size,
        source: metrics(&source),
        round_trip: metrics(&restored),
        output_sha256: crate::artifacts::digest(&output),
        output_size: output.len() as u64,
        preview_sha256: preview.as_ref().map(|v| crate::artifacts::digest(v)),
        preview_size: preview.as_ref().map(|v| v.len() as u64),
    };
    if !report.valid_for(request_id, direction, hash, size) || bytes(&input)? != original {
        return Err(error(
            "IGES_ROUND_TRIP_FAILED",
            "IGES exchange changed measurements, solids or sealed input",
        ));
    }
    fs::write(
        cwd.join("iges-report.json"),
        serde_json::to_vec(&report).map_err(|e| error("IGES_REPORT_INVALID", e.to_string()))?,
    )
    .map_err(|e| error("IGES_IO_FAILED", e.to_string()))?;
    Ok(report)
}
