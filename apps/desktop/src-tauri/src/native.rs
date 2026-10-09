//! Native OCCT boundary, enabled only when the matching OCCT development package is available.
pub mod assets;
mod chamfer;
mod document;
pub mod face_section;
mod fillet;
pub mod glb_merge;
mod hole;
pub mod inspection;
pub mod job;
pub mod pair_measurements;
pub mod reference_measurements;
mod revolution;
pub mod section;
pub mod step_import;
mod topology_refs;
mod transforms;
pub use document::{build_body, build_body_with_assets};
pub mod worker;
#[cxx::bridge(namespace = "forma")]
mod ffi {
    unsafe extern "C++" {
        include!("cad-core/include/forma_core.hpp");
        include!("cad-core/include/forma_measurement.hpp");
        include!("cad-core/include/forma_pair_measurement.hpp");

        type Shape;
        type MeasurementResult;
        type PairMeasurementResult;
        #[allow(clippy::too_many_arguments)]
        fn measure_reference_pair(
            source: &Shape,
            query_kind: i32,
            kind_a: i32,
            owner_a: &CxxString,
            role_a: &CxxString,
            path_a: &CxxString,
            kind_b: i32,
            owner_b: &CxxString,
            role_b: &CxxString,
            path_b: &CxxString,
        ) -> UniquePtr<PairMeasurementResult>;
        #[rust_name = "pair_measurement_valid"]
        fn valid(self: &PairMeasurementResult) -> bool;
        #[rust_name = "pair_query_kind"]
        fn query_kind(self: &PairMeasurementResult) -> i32;
        #[rust_name = "pair_distance_mm"]
        fn distance_mm(self: &PairMeasurementResult) -> f64;
        #[rust_name = "pair_angle_deg"]
        fn angle_deg(self: &PairMeasurementResult) -> f64;
        #[rust_name = "pair_point_a_mm"]
        fn point_a_mm(self: &PairMeasurementResult) -> &CxxVector<f64>;
        #[rust_name = "pair_point_b_mm"]
        fn point_b_mm(self: &PairMeasurementResult) -> &CxxVector<f64>;
        #[rust_name = "pair_measurement_error_code"]
        fn error_code(self: &PairMeasurementResult) -> &CxxString;
        #[rust_name = "pair_measurement_error_message"]
        fn error_message(self: &PairMeasurementResult) -> &CxxString;
        fn measure_reference(
            source: &Shape,
            query_kind: i32,
            owner: &CxxString,
            role: &CxxString,
            occurrence_path: &CxxString,
        ) -> UniquePtr<MeasurementResult>;
        #[rust_name = "measurement_valid"]
        fn valid(self: &MeasurementResult) -> bool;
        fn query_kind(self: &MeasurementResult) -> i32;
        fn length_mm(self: &MeasurementResult) -> f64;
        fn radius_mm(self: &MeasurementResult) -> f64;
        fn diameter_mm(self: &MeasurementResult) -> f64;
        fn area_mm2(self: &MeasurementResult) -> f64;
        #[rust_name = "measurement_volume_mm3"]
        fn volume_mm3(self: &MeasurementResult) -> f64;
        #[rust_name = "measurement_face_count"]
        fn face_count(self: &MeasurementResult) -> i32;
        #[rust_name = "measurement_edge_count"]
        fn edge_count(self: &MeasurementResult) -> i32;
        fn extents_mm(self: &MeasurementResult) -> &CxxVector<f64>;
        fn origin_mm(self: &MeasurementResult) -> &CxxVector<f64>;
        fn normal(self: &MeasurementResult) -> &CxxVector<f64>;
        #[rust_name = "measurement_error_code"]
        fn error_code(self: &MeasurementResult) -> &CxxString;
        #[rust_name = "measurement_error_message"]
        fn error_message(self: &MeasurementResult) -> &CxxString;
        type SectionResult;
        #[allow(clippy::too_many_arguments)]
        fn section_plane(
            source: &Shape,
            ox: f64,
            oy: f64,
            oz: f64,
            nx: f64,
            ny: f64,
            nz: f64,
            deflection: f64,
        ) -> UniquePtr<SectionResult>;
        fn valid(self: &SectionResult) -> bool;
        fn points_mm(self: &SectionResult) -> &CxxVector<f64>;
        fn edge_offsets(self: &SectionResult) -> &CxxVector<u32>;
        fn edge_lengths_mm(self: &SectionResult) -> &CxxVector<f64>;
        #[rust_name = "section_error_code"]
        fn error_code(self: &SectionResult) -> &CxxString;
        #[rust_name = "section_error_message"]
        fn error_message(self: &SectionResult) -> &CxxString;

        fn make_box(width_mm: f64, depth_mm: f64, height_mm: f64) -> UniquePtr<Shape>;
        fn make_referenced_box(
            width_mm: f64,
            depth_mm: f64,
            height_mm: f64,
            owner: &CxxString,
        ) -> UniquePtr<Shape>;
        fn topology_occurrence(source: &Shape, id: &CxxString) -> UniquePtr<Shape>;
        fn fillet_referenced_edge(
            source: &Shape,
            owner: &CxxString,
            role: &CxxString,
            path: &CxxString,
            radius_mm: f64,
        ) -> UniquePtr<Shape>;
        fn make_cylinder(radius_mm: f64, height_mm: f64) -> UniquePtr<Shape>;
        fn make_referenced_cylinder(
            radius_mm: f64,
            height_mm: f64,
            owner: &CxxString,
        ) -> UniquePtr<Shape>;
        fn make_sphere(radius_mm: f64) -> UniquePtr<Shape>;
        fn make_cone(bottom_radius_mm: f64, top_radius_mm: f64, height_mm: f64)
            -> UniquePtr<Shape>;
        unsafe fn make_polygon_prism(
            outline_xy: *const f64,
            coordinate_count: usize,
            plane: i32,
            origin_x_mm: f64,
            origin_y_mm: f64,
            origin_z_mm: f64,
            distance_mm: f64,
        ) -> UniquePtr<Shape>;
        #[allow(clippy::too_many_arguments)]
        // Explicit pointer lengths and workplane coordinates at the C++ boundary.
        unsafe fn make_profile_prism(
            coordinates: *const f64,
            coordinate_count: usize,
            offsets: *const usize,
            offset_count: usize,
            plane: i32,
            origin_x_mm: f64,
            origin_y_mm: f64,
            origin_z_mm: f64,
            distance_mm: f64,
        ) -> UniquePtr<Shape>;
        fn fillet_all_edges(source: &Shape, radius_mm: f64) -> UniquePtr<Shape>;
        #[allow(clippy::too_many_arguments)]
        unsafe fn make_profile_revolve(
            coordinates: *const f64,
            coordinate_count: usize,
            offsets: *const usize,
            offset_count: usize,
            plane: i32,
            origin_x_mm: f64,
            origin_y_mm: f64,
            origin_z_mm: f64,
            axis_origin_x_mm: f64,
            axis_origin_y_mm: f64,
            axis_origin_z_mm: f64,
            axis_x: f64,
            axis_y: f64,
            axis_z: f64,
            angle_deg: f64,
        ) -> UniquePtr<Shape>;
        fn fillet_edge(source: &Shape, edge_key: &CxxString, radius_mm: f64) -> UniquePtr<Shape>;
        fn chamfer_all_edges(source: &Shape, distance_mm: f64) -> UniquePtr<Shape>;
        fn translate(source: &Shape, x_mm: f64, y_mm: f64, z_mm: f64) -> UniquePtr<Shape>;
        #[allow(clippy::too_many_arguments)]
        fn rotate(
            source: &Shape,
            origin_x_mm: f64,
            origin_y_mm: f64,
            origin_z_mm: f64,
            axis_x: f64,
            axis_y: f64,
            axis_z: f64,
            angle_deg: f64,
        ) -> UniquePtr<Shape>;
        fn mirror(
            source: &Shape,
            origin_x_mm: f64,
            origin_y_mm: f64,
            origin_z_mm: f64,
            normal_x: f64,
            normal_y: f64,
            normal_z: f64,
        ) -> UniquePtr<Shape>;
        fn boolean_union(left: &Shape, right: &Shape) -> UniquePtr<Shape>;
        fn boolean_cut(left: &Shape, right: &Shape) -> UniquePtr<Shape>;
        fn boolean_intersect(left: &Shape, right: &Shape) -> UniquePtr<Shape>;
        fn make_compound(left: &Shape, right: &Shape) -> UniquePtr<Shape>;
        fn read_step(path: &CxxString) -> UniquePtr<Shape>;
        fn write_step(source: &Shape, path: &CxxString, error_code: Pin<&mut CxxString>) -> bool;
        fn write_glb(source: &Shape, path: &CxxString, error_code: Pin<&mut CxxString>) -> bool;
        fn ok(self: &Shape) -> bool;
        fn volume_mm3(self: &Shape) -> f64;
        fn surface_area_mm2(self: &Shape) -> f64;
        fn face_count(self: &Shape) -> i32;
        fn edge_count(self: &Shape) -> i32;
        fn extent_x_mm(self: &Shape) -> f64;
        fn extent_y_mm(self: &Shape) -> f64;
        fn extent_z_mm(self: &Shape) -> f64;
        fn error_code(self: &Shape) -> &CxxString;
        fn error_message(self: &Shape) -> &CxxString;
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Native CAD operation failed: {code}")]
pub struct NativeError {
    pub code: String,
    pub detail: String,
}

pub struct Solid(cxx::UniquePtr<ffi::Shape>);

impl Solid {
    fn checked(shape: cxx::UniquePtr<ffi::Shape>) -> Result<Self, NativeError> {
        let reference = shape.as_ref().ok_or_else(|| NativeError {
            code: "NATIVE_ALLOCATION_FAILED".into(),
            detail: "CAD kernel did not allocate a shape".into(),
        })?;
        if !reference.ok() {
            return Err(NativeError {
                code: reference.error_code().to_string_lossy().into_owned(),
                detail: reference.error_message().to_string_lossy().into_owned(),
            });
        }
        Ok(Self(shape))
    }

    fn as_ref(&self) -> &ffi::Shape {
        // Every Solid is constructed through checked(), which rejects a null pointer.
        self.0.as_ref().expect("checked native shape")
    }

    pub fn box_solid(width_mm: f64, depth_mm: f64, height_mm: f64) -> Result<Self, NativeError> {
        Self::checked(ffi::make_box(width_mm, depth_mm, height_mm))
    }

    pub fn cylinder(radius_mm: f64, height_mm: f64) -> Result<Self, NativeError> {
        Self::checked(ffi::make_cylinder(radius_mm, height_mm))
    }

    pub fn sphere(radius_mm: f64) -> Result<Self, NativeError> {
        Self::checked(ffi::make_sphere(radius_mm))
    }

    pub fn cone(
        bottom_radius_mm: f64,
        top_radius_mm: f64,
        height_mm: f64,
    ) -> Result<Self, NativeError> {
        Self::checked(ffi::make_cone(bottom_radius_mm, top_radius_mm, height_mm))
    }

    pub fn polygon_prism(
        sketch: &crate::cad_ir::sketch::SolvedSketch,
        distance_mm: f64,
    ) -> Result<Self, NativeError> {
        let plane = match sketch.plane {
            crate::cad_ir::SketchPlane::Xy => 0,
            crate::cad_ir::SketchPlane::Xz => 1,
            crate::cad_ir::SketchPlane::Yz => 2,
        };
        if sketch.holes_xy.is_empty() {
            return Self::checked(unsafe {
                ffi::make_polygon_prism(
                    sketch.outline_xy.as_ptr(),
                    sketch.outline_xy.len(),
                    plane,
                    sketch.origin_mm[0],
                    sketch.origin_mm[1],
                    sketch.origin_mm[2],
                    distance_mm,
                )
            });
        }
        let mut coordinates = sketch.outline_xy.clone();
        let mut offsets = vec![0, coordinates.len()];
        for hole in &sketch.holes_xy {
            coordinates.extend(hole);
            offsets.push(coordinates.len());
        }
        Self::checked(unsafe {
            ffi::make_profile_prism(
                coordinates.as_ptr(),
                coordinates.len(),
                offsets.as_ptr(),
                offsets.len(),
                plane,
                sketch.origin_mm[0],
                sketch.origin_mm[1],
                sketch.origin_mm[2],
                distance_mm,
            )
        })
    }

    pub fn translated(&self, x_mm: f64, y_mm: f64, z_mm: f64) -> Result<Self, NativeError> {
        Self::checked(ffi::translate(self.as_ref(), x_mm, y_mm, z_mm))
    }

    pub fn fillet_edge(&self, edge_key: &str, radius_mm: f64) -> Result<Self, NativeError> {
        cxx::let_cxx_string!(key = edge_key);
        Self::checked(ffi::fillet_edge(self.as_ref(), &key, radius_mm))
    }

    pub fn union(&self, other: &Self) -> Result<Self, NativeError> {
        Self::checked(ffi::boolean_union(self.as_ref(), other.as_ref()))
    }

    pub fn cut(&self, other: &Self) -> Result<Self, NativeError> {
        Self::checked(ffi::boolean_cut(self.as_ref(), other.as_ref()))
    }

    pub fn intersect(&self, other: &Self) -> Result<Self, NativeError> {
        Self::checked(ffi::boolean_intersect(self.as_ref(), other.as_ref()))
    }

    pub fn compound(&self, other: &Self) -> Result<Self, NativeError> {
        Self::checked(ffi::make_compound(self.as_ref(), other.as_ref()))
    }

    pub fn volume_mm3(&self) -> f64 {
        self.as_ref().volume_mm3()
    }

    pub fn surface_area_mm2(&self) -> f64 {
        self.as_ref().surface_area_mm2()
    }

    pub fn face_count(&self) -> i32 {
        self.as_ref().face_count()
    }

    pub fn edge_count(&self) -> i32 {
        self.as_ref().edge_count()
    }

    pub fn bounds_mm(&self) -> [f64; 3] {
        [
            self.as_ref().extent_x_mm(),
            self.as_ref().extent_y_mm(),
            self.as_ref().extent_z_mm(),
        ]
    }

    pub fn read_step(path: &std::path::Path) -> Result<Self, NativeError> {
        let path = path.to_str().ok_or_else(|| NativeError {
            code: "INVALID_PATH".into(),
            detail: "STEP path cannot be represented as UTF-8".into(),
        })?;
        cxx::let_cxx_string!(native_path = path);
        Self::checked(ffi::read_step(&native_path))
    }

    pub fn write_step(&self, path: &std::path::Path) -> Result<(), NativeError> {
        let path = path.to_str().ok_or_else(|| NativeError {
            code: "INVALID_PATH".into(),
            detail: "STEP path cannot be represented as UTF-8".into(),
        })?;
        cxx::let_cxx_string!(native_path = path);
        cxx::let_cxx_string!(code = "");
        if ffi::write_step(self.as_ref(), &native_path, code) {
            Ok(())
        } else {
            Err(NativeError {
                code: "STEP_EXPORT_FAILED".into(),
                detail: "STEP export failed".into(),
            })
        }
    }

    pub fn write_glb(&self, path: &std::path::Path) -> Result<(), NativeError> {
        let path = path.to_str().ok_or_else(|| NativeError {
            code: "INVALID_PATH".into(),
            detail: "Preview path cannot be represented as UTF-8".into(),
        })?;
        cxx::let_cxx_string!(native_path = path);
        cxx::let_cxx_string!(code = "");
        if ffi::write_glb(self.as_ref(), &native_path, code) {
            Ok(())
        } else {
            Err(NativeError {
                code: "PREVIEW_FAILED".into(),
                detail: "Native tessellation failed".into(),
            })
        }
    }
}

pub fn box_volume(width_mm: f64, depth_mm: f64, height_mm: f64) -> Result<f64, NativeError> {
    Ok(Solid::box_solid(width_mm, depth_mm, height_mm)?.volume_mm3())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cxx_bridge_builds_valid_brep() {
        assert!((box_volume(60.0, 40.0, 10.0).unwrap() - 24000.0).abs() < 0.001);
        assert_eq!(
            box_volume(0.0, 40.0, 10.0).unwrap_err().code,
            "INVALID_DIMENSION"
        );
        let stock = Solid::box_solid(60.0, 40.0, 10.0).unwrap();
        assert!((stock.surface_area_mm2() - 6800.0).abs() < 0.001);
        assert_eq!((stock.face_count(), stock.edge_count()), (6, 12));
        let tool = Solid::cylinder(5.0, 10.0).unwrap();
        let cut = stock.cut(&tool).unwrap();
        let expected = 24000.0 - 250.0 * std::f64::consts::PI;
        assert!((cut.volume_mm3() - expected).abs() < 0.01);
        assert!((cut.translated(5.0, 0.0, 0.0).unwrap().volume_mm3() - expected).abs() < 0.01);
        let path = std::env::temp_dir().join(format!("forma-native-{}.step", uuid::Uuid::new_v4()));
        cut.write_step(&path).unwrap();
        let imported = Solid::read_step(&path).unwrap();
        assert!((imported.volume_mm3() - cut.volume_mm3()).abs() < 0.01);
        std::fs::remove_file(path).unwrap();
    }
}
