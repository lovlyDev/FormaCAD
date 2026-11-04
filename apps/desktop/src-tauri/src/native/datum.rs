use super::{ffi, NativeError};
use crate::cad_ir::Document;

pub(crate) fn validate(document: &Document) -> Result<(), NativeError> {
    for feature in &document.features {
        if let Some((kind, data, len)) = feature.operation.datum_packet() {
            // SAFETY: the fixed array outlives the synchronous call, and len is 3 or 7.
            if !unsafe { ffi::validate_datum(kind, data.as_ptr(), len) } {
                return Err(NativeError {
                    code: "DATUM_INVALID".into(),
                    detail: feature.id.clone(),
                });
            }
        }
    }
    Ok(())
}
