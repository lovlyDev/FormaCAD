use super::{ffi, NativeError, Solid};
impl Solid {
    pub(super) fn mirror_features(
        &self,
        before: Option<&Self>,
        origin: [f64; 3],
        normal: [f64; 3],
    ) -> Result<Self, NativeError> {
        Self::checked(ffi::mirror_features(
            self.as_ref(),
            before.unwrap_or(self).as_ref(),
            before.is_some(),
            origin[0],
            origin[1],
            origin[2],
            normal[0],
            normal[1],
            normal[2],
        ))
    }
}
