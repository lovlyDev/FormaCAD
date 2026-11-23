use super::{
    failure,
    schema::{ImportedAssetSeal, MeasurementBinding, MAX_DOCUMENT_BYTES},
};
use crate::{
    cad_ir::Document,
    core::{AppState, Result},
    models::{ProjectFile, Revision},
    project_repository::backend::RepositoryRead,
};
use std::path::Path;

#[derive(Debug, PartialEq, Eq)]
struct PreviewSeal {
    name: String,
    sha256: String,
    size: u64,
}

pub(crate) struct CapturedMeasurement<'a> {
    pub snapshot: RepositoryRead<'a>,
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

async fn sealed_bytes(read: &RepositoryRead<'_>, file: &ProjectFile) -> Result<Vec<u8>> {
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
    let bytes = read
        .read_asset(file)
        .await
        .map_err(super::snapshot::asset_error)?;
    if bytes.len() as u64 != file.size || crate::artifacts::digest(&bytes) != hash {
        return Err(failure(
            "ASSET_CHECKSUM_MISMATCH",
            "Measurement attachment differs from its committed seal",
        ));
    }
    Ok(bytes)
}

fn validate_input(project_id: &str, expected_revision: &str, body_id: &str) -> Result<()> {
    crate::security::valid_id(project_id)?;
    crate::security::valid_id(expected_revision)?;
    if body_id.is_empty() || body_id.len() > 80 {
        return Err(failure(
            "INVALID_MEASUREMENT_QUERY",
            "Selected body identifier is invalid",
        ));
    }
    Ok(())
}

pub(crate) async fn capture<'a>(
    state: &'a AppState,
    project_id: &str,
    expected_revision: &str,
    body_id: &str,
) -> Result<CapturedMeasurement<'a>> {
    validate_input(project_id, expected_revision, body_id)?;
    let snapshot = super::snapshot::read(state, project_id).await?;
    capture_bound(snapshot, expected_revision, body_id).await
}

pub(crate) async fn capture_bound<'a>(
    snapshot: RepositoryRead<'a>,
    expected_revision: &str,
    body_id: &str,
) -> Result<CapturedMeasurement<'a>> {
    validate_input(&snapshot.project().id, expected_revision, body_id)?;
    let project = snapshot.project();
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
    let source_bytes = sealed_bytes(&snapshot, source).await?;
    let source_sha256 = crate::artifacts::digest(&source_bytes);
    let source_size = source.size;
    drop(source_bytes); // Queued requests retain seals, not a 40 MB STEP buffer.
    let preview = if let Some(name) = revision.preview.as_deref() {
        let file = project
            .files
            .iter()
            .find(|entry| entry.name == name)
            .ok_or_else(|| failure("ASSET_MISSING", "Committed preview is missing"))?;
        let bytes = sealed_bytes(&snapshot, file).await?;
        Some(PreviewSeal {
            name: name.into(),
            sha256: crate::artifacts::digest(&bytes),
            size: file.size,
        })
    } else {
        None
    };
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
        let bytes = sealed_bytes(&snapshot, file).await?;
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
    super::snapshot::recheck(&snapshot).await?;
    Ok(CapturedMeasurement {
        snapshot,
        revision,
        document,
        program_bytes,
        binding,
        preview,
    })
}

pub(crate) async fn source_bytes(captured: &CapturedMeasurement<'_>) -> Result<Vec<u8>> {
    let file = captured
        .snapshot
        .project()
        .files
        .iter()
        .find(|file| Some(&file.name) == captured.revision.source.as_ref())
        .ok_or_else(|| failure("ASSET_MISSING", "Captured exact source is missing"))?;
    sealed_bytes(&captured.snapshot, file).await
}

pub(crate) async fn recheck(captured: &mut CapturedMeasurement<'_>) -> Result<()> {
    super::snapshot::recheck_lifecycle(&captured.snapshot).await?;
    let latest = capture_bound(
        super::snapshot::refresh(&captured.snapshot).await?,
        &captured.binding.revision_id,
        &captured.binding.body_id,
    )
    .await?;
    super::snapshot::recheck_lifecycle(&captured.snapshot).await?;
    if latest.snapshot.project().id != captured.snapshot.project().id
        || latest.snapshot.task_scope_id() != captured.snapshot.task_scope_id()
        || latest.revision != captured.revision
        || latest.binding != captured.binding
        || latest.preview != captured.preview
    {
        return Err(failure(
            "MEASUREMENT_STALE",
            "Committed measurement inputs changed while the request was running",
        ));
    }
    // Only the validated IO metadata advances. Geometry bindings and the same
    // source/request family remain original; release the old large projection.
    captured.snapshot = latest.snapshot;
    Ok(())
}
