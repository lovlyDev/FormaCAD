use super::{ffi, NativeError, Solid};
use crate::cad_ir::{TopologyKind, TopologyReference};

impl Solid {
    pub fn chamfer_all_edges(&self, distance_mm: f64) -> Result<Self, NativeError> {
        Self::checked(ffi::chamfer_all_edges(self.as_ref(), distance_mm))
    }

    pub fn chamfer_referenced_edge(
        &self,
        reference: &TopologyReference,
        distance_mm: f64,
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
        Self::checked(ffi::chamfer_referenced_edge(
            self.as_ref(),
            &owner,
            &role,
            &path,
            distance_mm,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_edge_chamfer_is_validated_by_the_kernel() {
        let box_solid = Solid::box_solid(60.0, 40.0, 10.0).unwrap();
        let beveled = box_solid.chamfer_all_edges(2.0).unwrap();
        assert!(beveled.volume_mm3() > 0.0);
        assert!(beveled.volume_mm3() < box_solid.volume_mm3());
        assert_eq!(
            box_solid.chamfer_all_edges(0.0).err().unwrap().code,
            "INVALID_DIMENSION"
        );
        assert!(box_solid.chamfer_all_edges(100.0).is_err());
    }
}
