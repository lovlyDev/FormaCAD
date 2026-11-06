use super::{ffi, NativeError, Solid};
use crate::cad_ir::TopologyReference;

impl Solid {
    pub(super) fn extrude_face(
        &self,
        reference: &TopologyReference,
        distance_mm: f64,
    ) -> Result<Self, NativeError> {
        cxx::let_cxx_string!(owner = &reference.owner_feature_id);
        cxx::let_cxx_string!(role = &reference.role);
        cxx::let_cxx_string!(occurrence = reference.occurrence_path.join("/"));
        Self::checked(ffi::extrude_face(
            self.as_ref(),
            &owner,
            &role,
            &occurrence,
            distance_mm,
        ))
    }
}
