//! Bounded owner-qualified provenance references, never mesh ordinals.
use super::validation::valid_id;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const MAX_OCCURRENCE_PATH: usize = 64;
pub const MAX_BLEND_EDGES: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TopologyKind {
    Edge,
    Face,
    Vertex,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TopologyReference {
    pub schema_version: u32,
    pub kind: TopologyKind,
    pub owner_feature_id: String,
    pub role: String,
    pub occurrence_path: Vec<String>,
}

/// Box corners named by their x, y and z constructor faces, e.g. `box-vertex:xmin:ymax:zmax`.
pub fn valid_box_vertex_role(role: &str) -> bool {
    let parts: Vec<_> = role.split(':').collect();
    parts.len() == 4
        && parts[0] == "box-vertex"
        && matches!(parts[1], "xmin" | "xmax")
        && matches!(parts[2], "ymin" | "ymax")
        && matches!(parts[3], "zmin" | "zmax")
}

impl TopologyReference {
    pub fn is_valid(&self) -> bool {
        let mut unique = HashSet::new();
        self.schema_version == 1
            && valid_id(&self.owner_feature_id)
            && self.occurrence_path.len() <= MAX_OCCURRENCE_PATH
            && self
                .occurrence_path
                .iter()
                .all(|id| valid_id(id) && id != &self.owner_feature_id && unique.insert(id))
            && match self.kind {
                TopologyKind::Edge => {
                    super::topology::valid_box_edge_key(&self.role)
                        || matches!(
                            self.role.as_str(),
                            "cylinder-edge:bottom" | "cylinder-edge:top"
                        )
                }
                TopologyKind::Face => matches!(
                    self.role.as_str(),
                    "box-face:xmin"
                        | "box-face:xmax"
                        | "box-face:ymin"
                        | "box-face:ymax"
                        | "box-face:zmin"
                        | "box-face:zmax"
                        | "cylinder-face:bottom"
                        | "cylinder-face:top"
                        | "cylinder-face:side"
                ),
                TopologyKind::Vertex => valid_box_vertex_role(&self.role),
            }
    }
}

pub fn valid_blend_edges(first: &TopologyReference, additional: &[TopologyReference]) -> bool {
    let mut unique = HashSet::new();
    additional.len() < MAX_BLEND_EDGES
        && std::iter::once(first).chain(additional).all(|reference| {
            reference.kind == TopologyKind::Edge && reference.is_valid() && unique.insert(reference)
        })
}
