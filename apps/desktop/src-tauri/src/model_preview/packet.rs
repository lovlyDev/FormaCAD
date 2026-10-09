use crate::core::{AppError, Result};
use serde::{Deserialize, Serialize};
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewMetrics {
    pub volume_mm3: f64,
    pub area_mm2: f64,
    pub face_count: i32,
    pub edge_count: i32,
    pub bounds_mm: [f64; 3],
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Header<'a> {
    protocol_version: u32,
    source_sha256: &'a str,
    metrics: &'a PreviewMetrics,
}
/// Four-byte LE JSON-header size, bounded metadata JSON, then the native GLB bytes.
pub(super) fn encode(source_sha256: &str, metrics: &PreviewMetrics, glb: &[u8]) -> Result<Vec<u8>> {
    if ![metrics.volume_mm3, metrics.area_mm2]
        .into_iter()
        .chain(metrics.bounds_mm)
        .all(|v| v.is_finite() && v >= 0.0)
        || metrics.face_count <= 0
        || metrics.edge_count < 0
        || glb.len() > crate::artifacts::MAX_FILE_BYTES
        || glb.len() < 20
        || &glb[..4] != b"glTF"
    {
        return Err(AppError::Invalid(
            "Model preview response is invalid".into(),
        ));
    }
    let header = serde_json::to_vec(&Header {
        protocol_version: 1,
        source_sha256,
        metrics,
    })?;
    let mut packet = Vec::with_capacity(4 + header.len() + glb.len());
    packet.extend((header.len() as u32).to_le_bytes());
    packet.extend(header);
    packet.extend(glb);
    Ok(packet)
}
