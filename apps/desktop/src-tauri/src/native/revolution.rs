//! Revolution of a solved closed profile, including its authored inner loops.
use super::{ffi, NativeError, Solid};
use crate::cad_ir::{sketch::SolvedSketch, SketchPlane};

impl Solid {
    pub fn revolved_profile(
        sketch: &SolvedSketch,
        axis_origin_mm: [f64; 3],
        axis_direction: [f64; 3],
        angle_deg: f64,
    ) -> Result<Self, NativeError> {
        let mut coordinates = sketch.outline_xy.clone();
        let mut offsets = vec![0, coordinates.len()];
        for hole in &sketch.holes_xy {
            coordinates.extend(hole);
            offsets.push(coordinates.len());
        }
        let plane = match sketch.plane {
            SketchPlane::Xy => 0,
            SketchPlane::Xz => 1,
            SketchPlane::Yz => 2,
        };
        Self::checked(unsafe {
            ffi::make_profile_revolve(
                coordinates.as_ptr(),
                coordinates.len(),
                offsets.as_ptr(),
                offsets.len(),
                plane,
                sketch.origin_mm[0],
                sketch.origin_mm[1],
                sketch.origin_mm[2],
                axis_origin_mm[0],
                axis_origin_mm[1],
                axis_origin_mm[2],
                axis_direction[0],
                axis_direction[1],
                axis_direction[2],
                angle_deg,
            )
        })
    }
}
