use super::{ffi, NativeError, Solid};
use crate::cad_ir::TopologyReference;
use std::fmt::Write;

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
        self.blend_referenced_edges(reference, &[], radius, false)
    }

    pub fn blend_referenced_edges(
        &self,
        reference: &TopologyReference,
        additional: &[TopologyReference],
        size: f64,
        chamfer: bool,
    ) -> Result<Self, NativeError> {
        if !crate::cad_ir::topology_refs::valid_blend_edges(reference, additional) {
            return Err(NativeError {
                code: "INVALID_TOPOLOGY_REFERENCE".into(),
                detail: "Invalid bounded edge reference".into(),
            });
        }
        cxx::let_cxx_string!(owner = &reference.owner_feature_id);
        cxx::let_cxx_string!(role = &reference.role);
        cxx::let_cxx_string!(path = reference.occurrence_path.join("/"));
        // Identifier and role validation excludes these packet separators.
        let mut packet = String::new();
        for edge in additional {
            writeln!(
                packet,
                "{}\t{}\t{}",
                edge.owner_feature_id,
                edge.role,
                edge.occurrence_path.join("/")
            )
            .expect("writing to String cannot fail");
        }
        cxx::let_cxx_string!(packet = packet);
        Self::checked(ffi::blend_referenced_edges(
            self.as_ref(),
            &owner,
            &role,
            &path,
            &packet,
            size,
            chamfer,
        ))
    }
}
