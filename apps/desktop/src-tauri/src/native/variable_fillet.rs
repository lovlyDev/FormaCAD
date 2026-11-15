//! Exact endpoint binding and linear rolling-ball radius law, owned BREP input.
use super::{NativeError, Solid};
use crate::cad_ir::TopologyReference;

#[cxx::bridge(namespace = "forma")]
mod ffi {
    unsafe extern "C++" {
        include!("cad-core/include/forma_variable_fillet.hpp");
        type Shape = super::super::ffi::Shape;
        type EdgeEndpoints;
        fn variable_fillet(
            source: &Shape,
            owner: &CxxString,
            role: &CxxString,
            path: &CxxString,
            start: f64,
            end: f64,
        ) -> UniquePtr<Shape>;
        fn variable_fillet_endpoints(
            source: &Shape,
            owner: &CxxString,
            role: &CxxString,
            path: &CxxString,
        ) -> UniquePtr<EdgeEndpoints>;
        fn valid(self: &EdgeEndpoints) -> bool;
        fn start_mm(self: &EdgeEndpoints) -> &CxxVector<f64>;
        fn end_mm(self: &EdgeEndpoints) -> &CxxVector<f64>;
        fn error_code(self: &EdgeEndpoints) -> &CxxString;
        fn error_message(self: &EdgeEndpoints) -> &CxxString;
    }
}
impl Solid {
    pub fn variable_fillet(
        &self,
        reference: &TopologyReference,
        start: f64,
        end: f64,
    ) -> Result<Self, NativeError> {
        cxx::let_cxx_string!(owner = &reference.owner_feature_id);
        cxx::let_cxx_string!(role = &reference.role);
        cxx::let_cxx_string!(path = reference.occurrence_path.join("/"));
        Self::checked(ffi::variable_fillet(
            self.as_ref(),
            &owner,
            &role,
            &path,
            start,
            end,
        ))
    }
    pub(super) fn variable_fillet_endpoints(
        &self,
        reference: &TopologyReference,
    ) -> Result<([f64; 3], [f64; 3]), NativeError> {
        cxx::let_cxx_string!(owner = &reference.owner_feature_id);
        cxx::let_cxx_string!(role = &reference.role);
        cxx::let_cxx_string!(path = reference.occurrence_path.join("/"));
        let result = ffi::variable_fillet_endpoints(self.as_ref(), &owner, &role, &path);
        let result = result.as_ref().ok_or_else(|| NativeError {
            code: "VARIABLE_FILLET_FAILED".into(),
            detail: "Missing exact endpoints".into(),
        })?;
        if !result.valid() {
            return Err(NativeError {
                code: result.error_code().to_string_lossy().into_owned(),
                detail: result.error_message().to_string_lossy().into_owned(),
            });
        }
        let vector = |v: &cxx::CxxVector<f64>| -> Result<[f64; 3], NativeError> {
            v.iter()
                .copied()
                .collect::<Vec<_>>()
                .try_into()
                .map_err(|_| NativeError {
                    code: "MEASUREMENT_INVALID_RESULT".into(),
                    detail: "Invalid endpoint vector".into(),
                })
        };
        Ok((vector(result.start_mm())?, vector(result.end_mm())?))
    }
}
