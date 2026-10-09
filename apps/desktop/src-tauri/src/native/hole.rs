//! A named axial hole operation over an existing BREP feature.
use super::{NativeError, Solid};

pub(super) fn cut_hole(
    source: &Solid,
    center_mm: [f64; 2],
    start_z_mm: f64,
    radius_mm: f64,
    depth_mm: f64,
) -> Result<Solid, NativeError> {
    let cutter =
        Solid::cylinder(radius_mm, depth_mm)?.translated(center_mm[0], center_mm[1], start_z_mm)?;
    let result = source.cut(&cutter)?;
    let removed = source.volume_mm3() - result.volume_mm3();
    if removed <= source.volume_mm3().max(1.0) * 1e-10 {
        return Err(NativeError {
            code: "HOLE_NO_INTERSECTION".into(),
            detail: "The hole does not remove material from the target body".into(),
        });
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bore_removes_expected_material_and_rejects_missed_target() {
        let stock = Solid::box_solid(10.0, 10.0, 10.0).unwrap();
        let drilled = cut_hole(&stock, [0.0, 0.0], 0.0, 2.0, 10.0).unwrap();
        let expected = 1000.0 - 40.0 * std::f64::consts::PI;
        assert!((drilled.volume_mm3() - expected).abs() < 0.01);
        assert_eq!(
            cut_hole(&stock, [50.0, 50.0], 0.0, 2.0, 10.0)
                .err()
                .unwrap()
                .code,
            "HOLE_NO_INTERSECTION"
        );
        assert!((stock.volume_mm3() - 1000.0).abs() < 0.001);
    }
}
