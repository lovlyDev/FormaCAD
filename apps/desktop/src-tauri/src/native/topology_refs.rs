use super::{ffi, NativeError, Solid};
use crate::cad_ir::{TopologyKind, TopologyReference};

impl Solid {
    pub fn referenced_cylinder(radius: f64, height: f64, owner: &str) -> Result<Self, NativeError> {
        cxx::let_cxx_string!(owner = owner);
        Self::checked(ffi::make_referenced_cylinder(radius, height, &owner))
    }
    pub fn referenced_box(
        width: f64,
        depth: f64,
        height: f64,
        owner: &str,
    ) -> Result<Self, NativeError> {
        cxx::let_cxx_string!(owner = owner);
        Self::checked(ffi::make_referenced_box(width, depth, height, &owner))
    }
    pub fn with_topology_occurrence(&self, id: &str) -> Result<Self, NativeError> {
        cxx::let_cxx_string!(id = id);
        Self::checked(ffi::topology_occurrence(self.as_ref(), &id))
    }
    pub fn fillet_referenced_edge(
        &self,
        reference: &TopologyReference,
        radius: f64,
    ) -> Result<Self, NativeError> {
        if !reference.is_valid()
            || reference.kind != TopologyKind::Edge
            || !crate::cad_ir::topology::valid_box_edge_key(&reference.role)
        {
            return Err(NativeError {
                code: "INVALID_TOPOLOGY_REFERENCE".into(),
                detail: "Invalid bounded edge reference".into(),
            });
        }
        cxx::let_cxx_string!(owner = &reference.owner_feature_id);
        cxx::let_cxx_string!(role = &reference.role);
        cxx::let_cxx_string!(path = reference.occurrence_path.join("/"));
        Self::checked(ffi::fillet_referenced_edge(
            self.as_ref(),
            &owner,
            &role,
            &path,
            radius,
        ))
    }
}
