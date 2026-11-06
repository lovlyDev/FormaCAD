use super::{ffi, NativeError, Solid};
use crate::cad_ir::{shell::valid_openings, TopologyReference};

impl Solid {
    pub fn shell(
        &self,
        openings: &[TopologyReference],
        offset_mm: f64,
    ) -> Result<Self, NativeError> {
        if !valid_openings(openings) {
            return Err(NativeError {
                code: "INVALID_TOPOLOGY_REFERENCE".into(),
                detail: "Shell openings must be unique faces of the same occurrence".into(),
            });
        }
        let first = &openings[0];
        let roles = openings
            .iter()
            .map(|face| face.role.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        cxx::let_cxx_string!(owner = &first.owner_feature_id);
        cxx::let_cxx_string!(roles = roles);
        cxx::let_cxx_string!(occurrence = first.occurrence_path.join("/"));
        Self::checked(ffi::shell(
            self.as_ref(),
            &owner,
            &roles,
            &occurrence,
            offset_mm,
        ))
    }
}
