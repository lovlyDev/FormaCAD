//! Owned planar BREP profiles. No positive-volume Solid checks are weakened.
use super::{NativeError, Solid};
use crate::cad_ir::{offset_profile::ProfileJoin, sketch::SolvedSketch, SketchPlane};
#[cxx::bridge(namespace = "forma")]
mod ffi {
    unsafe extern "C++" {
        include!("cad-core/include/forma_offset_profile.hpp");
        type Shape = super::super::ffi::Shape;
        type PlanarProfile;
        #[allow(clippy::too_many_arguments)]
        unsafe fn make_planar_profile(
            c: *const f64,
            n: usize,
            offsets: *const usize,
            loops: usize,
            plane: i32,
            x: f64,
            y: f64,
            z: f64,
        ) -> UniquePtr<PlanarProfile>;
        fn make_circle_profile(radius: f64) -> UniquePtr<PlanarProfile>;
        fn offset_profile(
            source: &PlanarProfile,
            offset: f64,
            rounded: bool,
        ) -> UniquePtr<PlanarProfile>;
        fn valid(self: &PlanarProfile) -> bool;
        fn error_code(self: &PlanarProfile) -> &CxxString;
        fn error_message(self: &PlanarProfile) -> &CxxString;
        fn extrude_planar_profile(source: &PlanarProfile, distance: f64) -> UniquePtr<Shape>;
        #[allow(clippy::too_many_arguments)]
        fn revolve_planar_profile(
            source: &PlanarProfile,
            x: f64,
            y: f64,
            z: f64,
            nx: f64,
            ny: f64,
            nz: f64,
            degrees: f64,
        ) -> UniquePtr<Shape>;
    }
}
pub(super) struct Profile(cxx::UniquePtr<ffi::PlanarProfile>);
impl Profile {
    fn checked(value: cxx::UniquePtr<ffi::PlanarProfile>) -> Result<Self, NativeError> {
        let Some(profile) = value.as_ref() else {
            return Err(NativeError {
                code: "INVALID_PROFILE".into(),
                detail: "Missing native profile".into(),
            });
        };
        if !profile.valid() {
            return Err(NativeError {
                code: profile.error_code().to_string_lossy().into_owned(),
                detail: profile.error_message().to_string_lossy().into_owned(),
            });
        }
        Ok(Self(value))
    }
    fn as_ref(&self) -> &ffi::PlanarProfile {
        self.0.as_ref().expect("checked profile")
    }
    pub(super) fn rectangle(width: f64, depth: f64) -> Result<Self, NativeError> {
        let c = [
            -width / 2.,
            -depth / 2.,
            width / 2.,
            -depth / 2.,
            width / 2.,
            depth / 2.,
            -width / 2.,
            depth / 2.,
        ];
        let loops = [0, c.len()];
        Self::checked(unsafe {
            ffi::make_planar_profile(
                c.as_ptr(),
                c.len(),
                loops.as_ptr(),
                loops.len(),
                0,
                0.,
                0.,
                0.,
            )
        })
    }
    pub(super) fn circle(radius: f64) -> Result<Self, NativeError> {
        Self::checked(ffi::make_circle_profile(radius))
    }
    pub(super) fn sketch(sketch: &SolvedSketch) -> Result<Self, NativeError> {
        let mut c = sketch.outline_xy.clone();
        let mut offsets = vec![0, c.len()];
        for hole in &sketch.holes_xy {
            c.extend(hole);
            offsets.push(c.len());
        }
        let plane = match sketch.plane {
            SketchPlane::Xy => 0,
            SketchPlane::Xz => 1,
            SketchPlane::Yz => 2,
        };
        Self::checked(unsafe {
            ffi::make_planar_profile(
                c.as_ptr(),
                c.len(),
                offsets.as_ptr(),
                offsets.len(),
                plane,
                sketch.origin_mm[0],
                sketch.origin_mm[1],
                sketch.origin_mm[2],
            )
        })
    }
    pub(super) fn offset(&self, distance: f64, join: ProfileJoin) -> Result<Self, NativeError> {
        Self::checked(ffi::offset_profile(
            self.as_ref(),
            distance,
            join == ProfileJoin::Rounded,
        ))
    }
    pub(super) fn extrude(&self, distance: f64) -> Result<Solid, NativeError> {
        Solid::checked(ffi::extrude_planar_profile(self.as_ref(), distance))
    }
    pub(super) fn revolve(
        &self,
        origin: [f64; 3],
        axis: [f64; 3],
        angle: f64,
    ) -> Result<Solid, NativeError> {
        Solid::checked(ffi::revolve_planar_profile(
            self.as_ref(),
            origin[0],
            origin[1],
            origin[2],
            axis[0],
            axis[1],
            axis[2],
            angle,
        ))
    }
}
