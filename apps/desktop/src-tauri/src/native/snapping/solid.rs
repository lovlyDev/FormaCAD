use crate::{
    model_snap::schema::{SnapQuery, SnapValue},
    native::{ffi, NativeError, Solid},
};
pub(super) fn error(code: &str, detail: impl Into<String>) -> NativeError {
    NativeError {
        code: code.into(),
        detail: detail.into(),
    }
}
impl Solid {
    pub fn snap_reference(&self, query: &SnapQuery) -> Result<SnapValue, NativeError> {
        if !query.valid() {
            return Err(error("INVALID_SNAP_QUERY", "Invalid exact snap query"));
        }
        let reference = query.reference();
        cxx::let_cxx_string!(owner = reference.owner_feature_id.as_str());
        cxx::let_cxx_string!(role = reference.role.as_str());
        cxx::let_cxx_string!(path = reference.occurrence_path.join("/"));
        let ray = query.ray();
        let (pointer, count) = ray
            .as_ref()
            .map_or((std::ptr::null(), 0), |ray| (ray.as_ptr(), ray.len()));
        // SAFETY: the optional fixed-size ray remains live for this synchronous native call.
        let result = unsafe {
            ffi::snap_reference(
                self.as_ref(),
                query.tag(),
                &owner,
                &role,
                &path,
                pointer,
                count,
            )
        };
        let result = result
            .as_ref()
            .ok_or_else(|| error("SNAP_FAILED", "Native snap allocation failed"))?;
        if !result.snap_valid() {
            return Err(error(
                &result.snap_error_code().to_string_lossy(),
                result.snap_error_message().to_string_lossy(),
            ));
        }
        if result.snap_query_kind() != query.tag() {
            return Err(error(
                "SNAP_FAILED",
                "Native snap returned another query kind",
            ));
        }
        let point_mm = result
            .snap_point_mm()
            .iter()
            .copied()
            .collect::<Vec<_>>()
            .try_into()
            .map_err(|_| error("SNAP_FAILED", "Invalid native snap point"))?;
        let normal = if result.snap_normal().is_empty() {
            None
        } else {
            Some(
                result
                    .snap_normal()
                    .iter()
                    .copied()
                    .collect::<Vec<_>>()
                    .try_into()
                    .map_err(|_| error("SNAP_FAILED", "Invalid native snap normal"))?,
            )
        };
        let value = SnapValue { point_mm, normal };
        if !value.valid_for(query) {
            return Err(error("SNAP_FAILED", "Native snap result is outside bounds"));
        }
        Ok(value)
    }
}
