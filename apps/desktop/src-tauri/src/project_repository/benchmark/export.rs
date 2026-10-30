//! Real CADPACK codec comparison on disposable, captured synthetic sources.
//! The caller owns the destination; this does not publish a user export or
//! establish a production transaction between source and destination.
use crate::{
    cadpack::benchmark_api::{self, BundleProof},
    core::Result,
    project_repository::{folder::FolderRepository, location::LocationIdentity},
};
use std::path::Path;

pub(super) fn manifest(root: &Path, destination: &Path) -> Result<BundleProof> {
    let repository = FolderRepository::open(root)?;
    let captured = repository.capture()?;
    let history = repository.read_history(&captured)?;
    repository.recheck(&captured)?;
    let proof = benchmark_api::round_trip(
        captured.project(),
        destination,
        history.as_deref(),
        |file| repository.read_asset(&captured, file),
    )?;
    repository.recheck(&captured)?;
    Ok(proof)
}

pub(super) async fn sqlite(root: &Path, destination: &Path) -> Result<BundleProof> {
    let location = LocationIdentity::capture(root, Path::new(""))?;
    let captured = super::copy::read_sql(root).await?;
    if !location.recheck(root, Path::new(""))? {
        return Err(super::super::legacy::changed());
    }
    let proof = benchmark_api::round_trip(
        captured.project(),
        destination,
        Some(captured.archive()),
        |file| super::copy::read_asset(root, file),
    )?;
    let current = super::copy::read_sql(root).await?;
    if !captured.matches(&current) || !location.recheck(root, Path::new(""))? {
        return Err(super::super::legacy::changed());
    }
    Ok(proof)
}

pub(super) fn print_proof(size: &str, backend: &str, proof: &BundleProof) {
    println!(
        "STORAGE_BENCHMARK {}",
        serde_json::json!({
            "kind":"cadpack_proof", "size":size, "backend":backend,
            "project_sha256":proof.project_sha256, "history_sha256":proof.history_sha256,
            "attachment_references":proof.attachment_references, "unique_assets":proof.unique_assets,
            "unique_asset_bytes":proof.unique_asset_bytes, "asset_reader_calls":proof.asset_reader_calls,
            "archive_bytes":proof.archive_bytes,
            "note":"actual existing CADPACK writer and reader; original source rechecked; disposable synthetic destination"
        })
    );
}
