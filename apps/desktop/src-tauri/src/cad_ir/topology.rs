//! Deliberately narrow semantic selectors for full-span, axis-aligned edges.
//! Native regeneration resolves each selector uniquely or fails closed.

pub fn valid_box_edge_key(key: &str) -> bool {
    matches!(
        key,
        "box-edge:x:ymin:zmin"
            | "box-edge:x:ymin:zmax"
            | "box-edge:x:ymax:zmin"
            | "box-edge:x:ymax:zmax"
            | "box-edge:y:xmin:zmin"
            | "box-edge:y:xmin:zmax"
            | "box-edge:y:xmax:zmin"
            | "box-edge:y:xmax:zmax"
            | "box-edge:z:xmin:ymin"
            | "box-edge:z:xmin:ymax"
            | "box-edge:z:xmax:ymin"
            | "box-edge:z:xmax:ymax"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_supported_semantic_edges_are_accepted() {
        assert!(valid_box_edge_key("box-edge:x:ymin:zmin"));
        assert!(!valid_box_edge_key("box-edge:x:ymin:zmiddle"));
        assert!(!valid_box_edge_key("edge[3]"));
    }
}
