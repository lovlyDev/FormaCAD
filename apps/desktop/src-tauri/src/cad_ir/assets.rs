//! Content-addressed STEP references contain no filesystem locations.
use super::{Document, ErrorCode, Operation, Result, ValidationError};
use std::collections::BTreeMap;

pub const MAX_STEP_ASSETS: usize = 32;
pub fn valid_reference(asset_id: &str, sha256: &str) -> bool {
    sha256.len() == 64
        && sha256
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        && asset_id == format!("step_{sha256}")
}

/// Sorted unique references also serve project persistence/CADPACK integrity checks.
pub fn references(document: &Document) -> Result<Vec<(&str, &str)>> {
    let mut references = BTreeMap::new();
    for feature in &document.features {
        if let Operation::ImportStep { asset_id, sha256 } = &feature.operation {
            if !valid_reference(asset_id, sha256) {
                return Err(ValidationError::new(
                    ErrorCode::InvalidValue,
                    Some(feature.id.clone()),
                ));
            }
            references.insert(asset_id.as_str(), sha256.as_str());
        }
    }
    if references.len() > MAX_STEP_ASSETS {
        return Err(ValidationError::new(ErrorCode::InvalidDocument, None));
    }
    Ok(references.into_iter().collect())
}
