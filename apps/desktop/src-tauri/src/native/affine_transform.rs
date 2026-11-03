use super::{ffi, NativeError, Solid};

impl Solid {
    pub fn transformed(&self, matrix: &[f64; 12]) -> Result<Self, NativeError> {
        // SAFETY: a fixed-size live array and exact length are borrowed only for this call.
        Self::checked(unsafe {
            ffi::transform_affine(self.as_ref(), matrix.as_ptr(), matrix.len())
        })
    }
}
