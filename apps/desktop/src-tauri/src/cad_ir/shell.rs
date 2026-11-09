use super::{TopologyKind, TopologyReference};
use std::collections::HashSet;

pub(crate) fn valid_openings(openings: &[TopologyReference]) -> bool {
    let Some(first) = openings.first() else {
        return false;
    };
    if openings.len() > 16 {
        return false;
    }
    let mut roles = HashSet::new();
    openings.iter().all(|reference| {
        reference.kind == TopologyKind::Face
            && reference.is_valid()
            && reference.owner_feature_id == first.owner_feature_id
            && reference.occurrence_path == first.occurrence_path
            && roles.insert(reference.role.as_str())
    })
}
