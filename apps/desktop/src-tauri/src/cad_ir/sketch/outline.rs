use super::*;

pub(super) fn crosses(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> bool {
    let orient = |p: [f64; 2], q: [f64; 2], r: [f64; 2]| {
        (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0])
    };
    let on_segment = |p: [f64; 2], q: [f64; 2], r: [f64; 2]| {
        q[0] >= p[0].min(r[0]) - TOLERANCE
            && q[0] <= p[0].max(r[0]) + TOLERANCE
            && q[1] >= p[1].min(r[1]) - TOLERANCE
            && q[1] <= p[1].max(r[1]) + TOLERANCE
    };
    let (ab_c, ab_d, cd_a, cd_b) = (
        orient(a, b, c),
        orient(a, b, d),
        orient(c, d, a),
        orient(c, d, b),
    );
    const AREA_TOLERANCE: f64 = 1e-8;
    if (ab_c > AREA_TOLERANCE && ab_d < -AREA_TOLERANCE
        || ab_c < -AREA_TOLERANCE && ab_d > AREA_TOLERANCE)
        && (cd_a > AREA_TOLERANCE && cd_b < -AREA_TOLERANCE
            || cd_a < -AREA_TOLERANCE && cd_b > AREA_TOLERANCE)
    {
        return true;
    }
    (ab_c.abs() <= AREA_TOLERANCE && on_segment(a, c, b))
        || (ab_d.abs() <= AREA_TOLERANCE && on_segment(a, d, b))
        || (cd_a.abs() <= AREA_TOLERANCE && on_segment(c, a, d))
        || (cd_b.abs() <= AREA_TOLERANCE && on_segment(c, b, d))
}

pub(super) fn validate_outline(
    outline_xy: &[f64],
    feature_id: &str,
) -> Result<(), ValidationError> {
    let (vertex_pairs, remainder) = outline_xy.as_chunks::<2>();
    debug_assert!(remainder.is_empty());
    let vertices = vertex_pairs.to_vec();
    let mut twice_area = 0.0;
    for index in 0..vertices.len() {
        let next_index = (index + 1) % vertices.len();
        let a = vertices[index];
        let b = vertices[next_index];
        if (a[0] - b[0]).hypot(a[1] - b[1]) <= TOLERANCE {
            return Err(error(ErrorCode::InvalidSketch, feature_id));
        }
        let previous = vertices[(index + vertices.len() - 1) % vertices.len()];
        let incoming = [a[0] - previous[0], a[1] - previous[1]];
        let outgoing = [b[0] - a[0], b[1] - a[1]];
        if (incoming[0] * outgoing[1] - incoming[1] * outgoing[0]).abs() <= 1e-8
            && incoming[0] * outgoing[0] + incoming[1] * outgoing[1] < 0.0
        {
            return Err(error(ErrorCode::InvalidSketch, feature_id));
        }
        twice_area += a[0] * b[1] - b[0] * a[1];
        for other in index + 2..vertices.len() {
            if next_index == other || (other + 1) % vertices.len() == index {
                continue;
            }
            if crosses(
                a,
                b,
                vertices[other],
                vertices[(other + 1) % vertices.len()],
            ) {
                return Err(error(ErrorCode::InvalidSketch, feature_id));
            }
        }
    }
    if twice_area.abs() <= TOLERANCE {
        return Err(error(ErrorCode::InvalidSketch, feature_id));
    }
    Ok(())
}
