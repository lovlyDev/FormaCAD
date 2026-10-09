//! Rigid BREP modifiers. The kernel copies each input; original bodies are immutable.
use super::{ffi, NativeError, Solid};

impl Solid {
    pub fn rotated(
        &self,
        origin_mm: [f64; 3],
        direction: [f64; 3],
        angle_deg: f64,
    ) -> Result<Self, NativeError> {
        Self::checked(ffi::rotate(
            self.as_ref(),
            origin_mm[0],
            origin_mm[1],
            origin_mm[2],
            direction[0],
            direction[1],
            direction[2],
            angle_deg,
        ))
    }

    pub fn mirrored(&self, origin_mm: [f64; 3], normal: [f64; 3]) -> Result<Self, NativeError> {
        Self::checked(ffi::mirror(
            self.as_ref(),
            origin_mm[0],
            origin_mm[1],
            origin_mm[2],
            normal[0],
            normal[1],
            normal[2],
        ))
    }
}
