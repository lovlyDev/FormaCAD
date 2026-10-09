use super::{ffi, NativeError, Solid};

impl Solid {
    pub fn fillet_all_edges(&self, radius_mm: f64) -> Result<Self, NativeError> {
        Self::checked(ffi::fillet_all_edges(self.as_ref(), radius_mm))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_edge_fillet_is_validated_by_the_kernel() {
        let box_solid = Solid::box_solid(60.0, 40.0, 10.0).unwrap();
        let rounded = box_solid.fillet_all_edges(2.0).unwrap();
        assert!(rounded.volume_mm3() > 0.0);
        assert!(rounded.volume_mm3() < box_solid.volume_mm3());
        assert_eq!(
            box_solid.fillet_all_edges(0.0).err().unwrap().code,
            "INVALID_DIMENSION"
        );
        assert!(box_solid.fillet_all_edges(100.0).is_err());
    }
}
