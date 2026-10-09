//! Import history names a verified asset, never a filesystem path.
use crate::{
    cad_ir::{Body, Document, Feature, Operation},
    core::Result,
};
use sha2::{Digest, Sha256};

pub(super) fn source(revision_id: &str, name: &str, bytes: &[u8]) -> Result<String> {
    let sha256 = format!("{:x}", Sha256::digest(bytes));
    let document = Document {
        schema_version: 2,
        revision_id: revision_id.into(),
        parameters: Vec::new(),
        features: vec![Feature {
            id: "imported_step".into(),
            name: name.into(),
            suppressed: false,
            operation: Operation::ImportStep {
                asset_id: format!("step_{sha256}"),
                sha256,
            },
        }],
        bodies: vec![Body {
            id: "imported_body".into(),
            name: name.into(),
            source_feature_id: "imported_step".into(),
        }],
    };
    document.validate().map_err(crate::cad_ir::app_error)?;
    Ok(serde_json::to_string(&document)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn import_identity_tracks_bytes_and_contains_no_file_path() {
        let first: Document =
            serde_json::from_str(&source("revision", "Part.step", b"first").unwrap()).unwrap();
        let renamed: Document =
            serde_json::from_str(&source("revision", "Renamed.step", b"first").unwrap()).unwrap();
        let changed: Document =
            serde_json::from_str(&source("revision", "Part.step", b"changed").unwrap()).unwrap();
        assert_eq!(first.features[0].operation, renamed.features[0].operation);
        assert_ne!(first.features[0].operation, changed.features[0].operation);
        assert_eq!(first.bodies[0].source_feature_id, "imported_step");
        let operation = serde_json::to_value(&first.features[0].operation).unwrap();
        assert_eq!(operation.as_object().unwrap().len(), 3);
    }
}
