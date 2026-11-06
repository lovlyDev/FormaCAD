use super::{ffi, hole, NativeError, Solid};
use crate::cad_ir::{Dimension, Document, Operation};

pub(super) fn build_pattern<'a>(
    operation: &Operation,
    document: &Document,
    get_solid: impl Fn(&str) -> Result<&'a Solid, NativeError>,
    dimension: impl Fn(&Dimension) -> Result<f64, NativeError>,
) -> Result<Solid, NativeError> {
    let (target, repeat_hole) = match operation {
        Operation::PatternLinear {
            body_feature_id,
            repeat_hole,
            ..
        }
        | Operation::PatternCircular {
            body_feature_id,
            repeat_hole,
            ..
        } => (body_feature_id, *repeat_hole),
        _ => return Err(invalid_pattern("Expected a pattern operation")),
    };
    let place = |source: &Solid| match operation {
        Operation::PatternLinear { layout, .. } => {
            let [ux, uy, uz] = layout.step_u_mm;
            let [vx, vy, vz] = layout.step_v_mm;
            Solid::checked(ffi::pattern_linear(
                source.as_ref(),
                layout.count_u as i32,
                ux,
                uy,
                uz,
                layout.count_v as i32,
                vx,
                vy,
                vz,
            ))
        }
        Operation::PatternCircular { layout, .. } => {
            let [ox, oy, oz] = layout.axis_origin_mm;
            let [dx, dy, dz] = layout.axis_direction;
            Solid::checked(ffi::pattern_circular(
                source.as_ref(),
                layout.count as i32,
                ox,
                oy,
                oz,
                dx,
                dy,
                dz,
                layout.step_deg,
            ))
        }
        _ => Err(invalid_pattern("Expected a pattern operation")),
    };
    if !repeat_hole {
        return place(get_solid(target)?);
    }
    let source = document
        .features
        .iter()
        .find(|feature| feature.id == *target && !feature.suppressed);
    let Some(Operation::Hole {
        body_feature_id,
        center_mm,
        start_z_mm,
        radius,
        depth,
    }) = source.map(|feature| &feature.operation)
    else {
        return Err(invalid_pattern(
            "Only an active hole feature can be repeated",
        ));
    };
    let cutter = hole::hole_cutter(
        *center_mm,
        *start_z_mm,
        dimension(radius)?,
        dimension(depth)?,
    )?;
    let cutters = place(&cutter)?;
    let result = hole::cut_hole_geometry(get_solid(body_feature_id)?, &cutters)?;
    let original = get_solid(target)?.volume_mm3();
    if original - result.volume_mm3() <= original.max(1.0) * 1e-10 {
        return Err(invalid_pattern(
            "Repeated holes did not remove additional material",
        ));
    }
    Ok(result)
}

fn invalid_pattern(detail: &str) -> NativeError {
    NativeError {
        code: "PATTERN_INVALID".into(),
        detail: detail.into(),
    }
}
