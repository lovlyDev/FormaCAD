use super::{ffi, NativeError, Solid};
use crate::cad_ir::TopologyReference;

impl Solid {
    pub(super) fn thicken_face(
        &self,
        reference: &TopologyReference,
        offset_mm: f64,
    ) -> Result<Self, NativeError> {
        cxx::let_cxx_string!(owner = &reference.owner_feature_id);
        cxx::let_cxx_string!(role = &reference.role);
        cxx::let_cxx_string!(occurrence = reference.occurrence_path.join("/"));
        Self::checked(ffi::thicken_face(
            self.as_ref(),
            &owner,
            &role,
            &occurrence,
            offset_mm,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cad_ir::TopologyKind;
    #[test]
    fn thicken_bridge_keeps_the_source_and_rejects_zero_thickness() {
        let source = Solid::referenced_box(40., 20., 10., "pad").unwrap();
        let reference = TopologyReference {
            schema_version: 1,
            kind: TopologyKind::Face,
            owner_feature_id: "pad".into(),
            role: "box-face:zmax".into(),
            occurrence_path: vec![],
        };
        let result = source.thicken_face(&reference, 2.).unwrap();
        assert!((result.volume_mm3() - 1600.).abs() < 1e-6);
        assert!((result.surface_area_mm2() - 1840.).abs() < 1e-6);
        assert!((source.volume_mm3() - 8000.).abs() < 1e-6);
        assert_eq!(
            source.thicken_face(&reference, 0.).err().unwrap().code,
            "INVALID_THICKNESS"
        );
    }
}
