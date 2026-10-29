//! Worker-only exact OCCT section construction.
use super::{error, SectionCurve, SectionGeometry, SectionPlane};
use crate::native::{ffi, NativeError, Solid};

impl Solid {
    pub fn section(&self, plane: &SectionPlane) -> Result<SectionGeometry, NativeError> {
        let plane = plane.normalized()?;
        let origin = plane.origin_mm;
        let normal = plane.normal;
        let result = ffi::section_plane(
            self.as_ref(),
            origin[0],
            origin[1],
            origin[2],
            normal[0],
            normal[1],
            normal[2],
            plane.deflection_mm,
        );
        let result = result
            .as_ref()
            .ok_or_else(|| error("SECTION_FAILED", "Native section result is missing"))?;
        if !result.valid() {
            return Err(error(
                &result.section_error_code().to_string_lossy(),
                result
                    .section_error_message()
                    .to_string_lossy()
                    .into_owned(),
            ));
        }
        let points = result.points_mm();
        let offsets = result.edge_offsets();
        let lengths = result.edge_lengths_mm();
        if points.len() % 3 != 0
            || points.len() / 3 > 100000
            || lengths.len() > 4096
            || offsets.len() != lengths.len() + 1
            || offsets.get(0) != Some(&0)
        {
            return Err(error(
                "SECTION_LIMIT",
                "Native section data exceeds bounded layout",
            ));
        }
        let mut curves = Vec::with_capacity(lengths.len());
        let mut total = 0.;
        for index in 0..lengths.len() {
            let start = *offsets.get(index).unwrap() as usize;
            let end = *offsets.get(index + 1).unwrap() as usize;
            let length = *lengths.get(index).unwrap();
            if end > points.len() / 3 || end <= start + 1 || !length.is_finite() || length <= 0. {
                return Err(error("SECTION_FAILED", "Native section curve is invalid"));
            }
            let mut coordinates = Vec::with_capacity(end - start);
            for i in start..end {
                let p = [
                    *points.get(i * 3).unwrap(),
                    *points.get(i * 3 + 1).unwrap(),
                    *points.get(i * 3 + 2).unwrap(),
                ];
                if p.iter().any(|x| !x.is_finite()) {
                    return Err(error("SECTION_FAILED", "Native section points are invalid"));
                }
                coordinates.push(p);
            }
            let first = coordinates[0];
            let last = *coordinates.last().unwrap();
            let closed = first
                .into_iter()
                .zip(last)
                .map(|(a, b)| (a - b).powi(2))
                .sum::<f64>()
                <= 1e-12;
            total += length;
            curves.push(SectionCurve {
                id: format!("section-edge-{}", index + 1),
                length_mm: length,
                points_mm: coordinates,
                closed,
            });
        }
        if !total.is_finite() {
            return Err(error(
                "SECTION_LIMIT",
                "Native section total length is invalid",
            ));
        }
        Ok(SectionGeometry {
            plane,
            curves,
            total_length_mm: total,
        })
    }
}
