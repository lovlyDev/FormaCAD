//! Bounded owner-qualified provenance references, never mesh ordinals.
use super::validation::valid_id;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const MAX_OCCURRENCE_PATH: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TopologyKind {
    Edge,
    Face,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TopologyReference {
    pub schema_version: u32,
    pub kind: TopologyKind,
    pub owner_feature_id: String,
    pub role: String,
    pub occurrence_path: Vec<String>,
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
            }
    }
}
