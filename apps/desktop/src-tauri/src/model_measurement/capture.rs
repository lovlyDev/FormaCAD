use super::{
    failure,
    schema::{ImportedAssetSeal, MeasurementBinding, MAX_DOCUMENT_BYTES},
};
use crate::{
    cad_ir::Document,
    core::{AppState, Result},
    models::{Project, ProjectFile, Revision},
};
use std::path::Path;

#[derive(Debug, PartialEq, Eq)]
struct PreviewSeal {
    name: String,
    sha256: String,
    size: u64,
}

pub(crate) struct CapturedMeasurement {
    pub project: Project,
    pub revision: Revision,
    pub document: Document,
    pub program_bytes: Vec<u8>,
    pub binding: MeasurementBinding,
    preview: Option<PreviewSeal>,
}

fn is_step(file: &ProjectFile) -> bool {
    Path::new(&file.name)
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| {
            value.eq_ignore_ascii_case("step") || value.eq_ignore_ascii_case("stp")
        })
}

fn sealed_bytes(state: &AppState, project: &Project, file: &ProjectFile) -> Result<Vec<u8>> {
    let hash = file
        .sha256
        .as_deref()
        .filter(|value| crate::artifacts::valid_digest(value))
        .ok_or_else(|| {
            failure(
                "ASSET_INVALID",
                "Measurement input has no committed checksum",
            )
        })?;
    if file.size == 0 || file.size > crate::artifacts::MAX_FILE_BYTES as u64 {
        return Err(failure("ASSET_LIMIT", "Measurement input size is invalid"));
    }
    if file.data.is_none() {
        crate::security::file_name(&file.name)?;
        let extension = Path::new(&file.name)
            .extension()
            .and_then(|value| value.to_str())
            .ok_or_else(|| {
                failure(
                    "ASSET_INVALID",
                    "Measurement attachment extension is missing",
                )
            })?
            .to_ascii_lowercase();
        let path = crate::security::guarded(
            &state.root,
            Path::new(&format!("{}/attachments/{hash}.{extension}", project.id)),
        )?;
        let metadata = std::fs::metadata(path)?;
        if !metadata.is_file() || metadata.len() != file.size {
            return Err(failure(
                "ASSET_CHECKSUM_MISMATCH",
                "Measurement attachment size differs from its seal",
            ));
        }
    }
    let bytes = crate::artifacts::read(state, &project.id, file)
        .map_err(|error| failure("ASSET_CHECKSUM_MISMATCH", error.to_string()))?;
    if bytes.len() as u64 != file.size || crate::artifacts::digest(&bytes) != hash {
        return Err(failure(
            "ASSET_CHECKSUM_MISMATCH",
            "Measurement attachment differs from its committed seal",
        ));
    }
    Ok(bytes)
}

pub(crate) async fn capture(
    state: &AppState,
    project_id: &str,
    expected_revision: &str,
    body_id: &str,
) -> Result<CapturedMeasurement> {
    crate::security::valid_id(project_id)?;
    crate::security::valid_id(expected_revision)?;
    if body_id.is_empty() || body_id.len() > 80 {
        return Err(failure(
            "INVALID_MEASUREMENT_QUERY",
            "Selected body identifier is invalid",
        ));
    }
    let project = super::snapshot::read(state, project_id).await?;
    if project.current_revision.as_deref() != Some(expected_revision) {
        return Err(failure(
            "MEASUREMENT_STALE",
            "Measurement revision is no longer current",
        ));
    }
    let revision = project
        .revisions
        .iter()
        .find(|entry| entry.id == expected_revision)
        .cloned()
        .ok_or_else(|| failure("MEASUREMENT_STALE", "Measurement revision is missing"))?;
    let program = revision.program.as_deref().ok_or_else(|| {
        failure(
            "MEASUREMENT_UNSUPPORTED",
            "Current revision has no authored CAD IR v2",
        )
    })?;
    if program.is_empty() || program.len() > MAX_DOCUMENT_BYTES {
        return Err(failure(
            "INVALID_MEASUREMENT_QUERY",
            "Committed CAD document exceeds measurement bounds",
        ));
    }
    let document: Document = serde_json::from_str(program).map_err(|_| {
        failure(
            "MEASUREMENT_UNSUPPORTED",
            "Current revision is not authored CAD IR v2",
        )
    })?;
    document.validate().map_err(crate::cad_ir::app_error)?;
    if document.revision_id != expected_revision
        || !document.bodies.iter().any(|body| body.id == body_id)
    {
        return Err(failure(
            "INVALID_MEASUREMENT_QUERY",
            "Selected body or authored revision does not match the committed model",
        ));
    }
    let source = project
        .files
        .iter()
        .find(|file| Some(&file.name) == revision.source.as_ref() && is_step(file))
        .ok_or_else(|| {
            failure(
                "ASSET_MISSING",
                "Measurement requires a committed exact STEP source",
            )
        })?;
    let source_bytes = sealed_bytes(state, &project, source)?;
    let source_sha256 = crate::artifacts::digest(&source_bytes);
    let source_size = source.size;
    drop(source_bytes); // Queued requests retain seals, not a 40 MB STEP buffer.
    let preview = revision
        .preview
        .as_deref()
        .map(|name| {
            let file = project
                .files
                .iter()
                .find(|entry| entry.name == name)
                .ok_or_else(|| failure("ASSET_MISSING", "Committed preview is missing"))?;
            let bytes = sealed_bytes(state, &project, file)?;
            Ok::<_, crate::core::AppError>(PreviewSeal {
                name: name.into(),
                sha256: crate::artifacts::digest(&bytes),
                size: file.size,
            })
        })
        .transpose()?;
    let mut imported_asset_seals = Vec::new();
    let mut total = 0usize;
    for (asset_id, hash) in
        crate::cad_ir::assets::references(&document).map_err(crate::cad_ir::app_error)?
    {
        let file = project
            .files
            .iter()
            .find(|file| file.sha256.as_deref() == Some(hash) && is_step(file))
            .ok_or_else(|| failure("ASSET_MISSING", "Authored imported STEP is missing"))?;
        let bytes = sealed_bytes(state, &project, file)?;
        total = total
            .checked_add(bytes.len())
            .ok_or_else(|| failure("ASSET_LIMIT", "Measurement STEP input size overflow"))?;
        if total > crate::native::assets::MAX_TOTAL_STEP_BYTES {
            return Err(failure(
                "ASSET_LIMIT",
                "Measurement imported STEP inputs exceed 128 MB",
            ));
        }
        imported_asset_seals.push(ImportedAssetSeal {
            asset_id: asset_id.into(),
            sha256: hash.into(),
            size: file.size,
        });
    }
    let program_bytes = program.as_bytes().to_vec();
    let binding = MeasurementBinding {
        revision_id: expected_revision.into(),
        body_id: body_id.into(),
        source_sha256,
        source_size,
        document_sha256: crate::artifacts::digest(&program_bytes),
        imported_asset_seals,
    };
    if !binding.valid() {
        return Err(failure(
            "INVALID_MEASUREMENT_QUERY",
            "Captured input binding exceeds measurement limits",
        ));
    }
    Ok(CapturedMeasurement {
        project,
        revision,
        document,
        program_bytes,
        binding,
        preview,
    })
}

pub(crate) fn source_bytes(state: &AppState, captured: &CapturedMeasurement) -> Result<Vec<u8>> {
    let file = captured
        .project
        .files
        .iter()
        .find(|file| Some(&file.name) == captured.revision.source.as_ref())
        .ok_or_else(|| failure("ASSET_MISSING", "Captured exact source is missing"))?;
    sealed_bytes(state, &captured.project, file)
}

pub(crate) async fn recheck(state: &AppState, captured: &CapturedMeasurement) -> Result<()> {
    let latest = capture(
        state,
        &captured.project.id,
        &captured.binding.revision_id,
        &captured.binding.body_id,
    )
    .await?;
    if latest.revision != captured.revision
        || latest.binding != captured.binding
        || latest.preview != captured.preview
    {
        return Err(failure(
            "MEASUREMENT_STALE",
            "Committed measurement inputs changed while the request was running",
        ));
    }
    Ok(())
}
