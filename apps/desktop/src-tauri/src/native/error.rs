#[derive(Debug, thiserror::Error)]
#[error("Native CAD operation failed: {code}")]
pub struct NativeError {
    pub code: String,
    pub detail: String,
}
