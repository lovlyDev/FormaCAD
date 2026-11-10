use super::{ffi, NativeError, Solid};
use crate::cad_ir::split::SplitSide;
impl Solid {
    pub(super) fn split_by_plane(
        &self,
        origin: [f64; 3],
        normal: [f64; 3],
        side: SplitSide,
    ) -> Result<Self, NativeError> {
        Self::checked(ffi::split_by_plane(
            self.as_ref(),
            origin[0],
            origin[1],
            origin[2],
            normal[0],
            normal[1],
            normal[2],
            side == SplitSide::Positive,
        ))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn split_bridge_preserves_volume_and_rejects_a_missed_plane() {
        let source = Solid::box_solid(40., 20., 10.).unwrap();
        let positive = source
            .split_by_plane([5., 0., 0.], [1., 0., 0.], SplitSide::Positive)
            .unwrap();
        let negative = source
            .split_by_plane([5., 0., 0.], [1., 0., 0.], SplitSide::Negative)
            .unwrap();
        assert!((positive.volume_mm3() - 3000.).abs() < 1e-6);
        assert!((negative.volume_mm3() - 5000.).abs() < 1e-6);
        assert_eq!((positive.face_count(), negative.face_count()), (6, 6));
        assert_eq!(
            source
                .split_by_plane([100., 0., 0.], [1., 0., 0.], SplitSide::Positive)
                .err()
                .unwrap()
                .code,
            "SPLIT_NO_INTERSECTION"
        );
    }
}
