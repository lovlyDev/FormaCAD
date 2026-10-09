use super::*;

pub(super) const MAX_LOOPS: usize = 8;

/// Preserve the document's line order for deterministic loop/vertex identities.
pub(super) fn profile_loops<'a>(
    lines: &'a [SketchLine],
    feature_id: &str,
) -> Result<Vec<Vec<&'a str>>, ValidationError> {
    let profile: Vec<_> = lines.iter().filter(|line| !line.construction).collect();
    let next: HashMap<_, _> = profile
        .iter()
        .map(|line| (line.start_point_id.as_str(), line.end_point_id.as_str()))
        .collect();
    let mut seen = HashSet::new();
    let mut loops = Vec::new();
    for line in profile {
        let first = line.start_point_id.as_str();
        if seen.contains(first) {
            continue;
        }
        let mut cursor = first;
        let mut vertices = Vec::new();
        loop {
            if !seen.insert(cursor) {
                return Err(error(ErrorCode::InvalidSketch, feature_id));
            }
            vertices.push(cursor);
            cursor = *next
                .get(cursor)
                .ok_or_else(|| error(ErrorCode::InvalidSketch, feature_id))?;
            if cursor == first {
                break;
            }
        }
        if vertices.len() < 3 || loops.len() == MAX_LOOPS {
            return Err(error(ErrorCode::InvalidSketch, feature_id));
        }
        loops.push(vertices);
    }
    if loops.is_empty() {
        return Err(error(ErrorCode::InvalidSketch, feature_id));
    }
    Ok(loops)
}

fn area(loop_xy: &[f64]) -> f64 {
    let points: Vec<_> = loop_xy.chunks_exact(2).map(|p| [p[0], p[1]]).collect();
    (0..points.len())
        .map(|i| {
            let a = points[i];
            let b = points[(i + 1) % points.len()];
            a[0] * b[1] - b[0] * a[1]
        })
        .sum::<f64>()
        / 2.0
}

fn contains(loop_xy: &[f64], point: [f64; 2]) -> bool {
    let vertices: Vec<_> = loop_xy.chunks_exact(2).map(|p| [p[0], p[1]]).collect();
    let mut inside = false;
    for i in 0..vertices.len() {
        let a = vertices[i];
        let b = vertices[(i + 1) % vertices.len()];
        if (a[1] > point[1]) != (b[1] > point[1])
            && point[0] < (b[0] - a[0]) * (point[1] - a[1]) / (b[1] - a[1]) + a[0]
        {
            inside = !inside;
        }
    }
    inside
}

/// One outer boundary with non-touching, unnested holes. Islands require separate bodies.
pub(super) fn classify(
    mut loops: Vec<Vec<f64>>,
    feature_id: &str,
) -> Result<(Vec<f64>, Vec<Vec<f64>>), ValidationError> {
    for outline in &loops {
        outline::validate_outline(outline, feature_id)?;
    }
    for left in 0..loops.len() {
        for right in left + 1..loops.len() {
            let a: Vec<_> = loops[left].chunks_exact(2).map(|p| [p[0], p[1]]).collect();
            let b: Vec<_> = loops[right].chunks_exact(2).map(|p| [p[0], p[1]]).collect();
            for i in 0..a.len() {
                for j in 0..b.len() {
                    if outline::crosses(a[i], a[(i + 1) % a.len()], b[j], b[(j + 1) % b.len()]) {
                        return Err(error(ErrorCode::InvalidSketch, feature_id));
                    }
                }
            }
        }
    }
    let outer_index = (0..loops.len())
        .max_by(|&a, &b| area(&loops[a]).abs().total_cmp(&area(&loops[b]).abs()))
        .unwrap();
    let mut outer = loops.remove(outer_index);
    for (i, hole) in loops.iter().enumerate() {
        let point = [hole[0], hole[1]];
        if !contains(&outer, point)
            || loops
                .iter()
                .enumerate()
                .any(|(j, other)| j != i && contains(other, point))
        {
            return Err(error(ErrorCode::InvalidSketch, feature_id));
        }
    }
    // Canonical winding makes workplane-independent OpenCascade face construction deterministic.
    let reverse = |outline: &mut Vec<f64>| {
        let mut vertices: Vec<_> = outline.chunks_exact(2).map(|p| [p[0], p[1]]).collect();
        vertices[1..].reverse();
        *outline = vertices.into_iter().flatten().collect();
    };
    if area(&outer) < 0.0 {
        reverse(&mut outer);
    }
    for hole in &mut loops {
        if area(hole) > 0.0 {
            reverse(hole);
        }
    }
    Ok((outer, loops))
}

#[cfg(test)]
#[path = "profile_tests.rs"]
mod tests;
