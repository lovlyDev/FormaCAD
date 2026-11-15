//! An IGES import is an explicit new revision backed by a canonical STEP asset.
use crate::{
    cad_ir::{Body, Document, Feature, Operation},
    core::Result,
};
pub fn source(revision_id: &str, name: &str, step: &[u8]) -> Result<String> {
    let sha256 = crate::artifacts::digest(step);
    let document = Document {
        schema_version: 2,
        revision_id: revision_id.into(),
        parameters: Vec::new(),
        groups: Vec::new(),
        features: vec![Feature {
            id: "imported_iges".into(),
            name: name.into(),
            suppressed: false,
            operation: Operation::ImportStep {
                asset_id: format!("step_{sha256}"),
                sha256,
            },
        }],
        bodies: vec![Body {
            id: super::schema::IMPORTED_BODY_ID.into(),
            name: name.into(),
            source_feature_id: "imported_iges".into(),
        }],
    };
    document.validate().map_err(crate::cad_ir::app_error)?;
    Ok(serde_json::to_string(&document)?)
}
