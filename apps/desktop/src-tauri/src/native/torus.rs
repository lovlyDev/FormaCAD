use super::{ffi, NativeError, Solid};

impl Solid {
    pub(super) fn torus(major_radius_mm: f64, minor_radius_mm: f64) -> Result<Self, NativeError> {
        Self::checked(ffi::make_torus(major_radius_mm, minor_radius_mm))
    }
}
