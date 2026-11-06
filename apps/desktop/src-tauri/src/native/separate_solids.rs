use super::{ffi, NativeError, Solid};

impl Solid {
    pub(super) fn extract_solid(&self, index: u8, expected_count: u8) -> Result<Self, NativeError> {
        Self::checked(ffi::extract_solid(
            self.as_ref(),
            i32::from(index),
            i32::from(expected_count),
        ))
    }
}
