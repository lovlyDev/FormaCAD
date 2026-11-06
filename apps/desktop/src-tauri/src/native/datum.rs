use super::{ffi, NativeError, Solid};
use crate::cad_ir::{
    datum_anchor::DatumSource, Document, Operation, TopologyKind, TopologyReference,
};
use std::collections::BTreeMap;

pub(crate) fn validate(document: &Document) -> Result<(), NativeError> {
    resolve(document, |_| None).map(|_| ())
}

type Frame = ([f64; 3], [f64; 3]);

fn topology_frame(solid: &Solid, reference: &TopologyReference) -> Option<Frame> {
    let kind = match reference.kind {
        TopologyKind::Face => 0,
        TopologyKind::Edge => 1,
        _ => return None,
    };
    cxx::let_cxx_string!(owner = &reference.owner_feature_id);
    cxx::let_cxx_string!(role = &reference.role);
    cxx::let_cxx_string!(occurrence = reference.occurrence_path.join("/"));
    let mut frame = [0.; 6];
    // SAFETY: the six-value output array and borrowed inputs outlive the synchronous call.
    if !unsafe {
        ffi::datum_anchor(
            solid.as_ref(),
            kind,
            &owner,
            &role,
            &occurrence,
            frame.as_mut_ptr(),
            frame.len(),
        )
    } {
        return None;
    }
    Some((
        [frame[0], frame[1], frame[2]],
        [frame[3], frame[4], frame[5]],
    ))
}

pub(super) fn resolve<'a, 'solid>(
    document: &'a Document,
    lookup: impl Fn(&str) -> Option<&'solid Solid>,
) -> Result<BTreeMap<&'a str, Operation>, NativeError> {
    let mut placed = BTreeMap::new();
    for feature in &document.features {
        let operation = match &feature.operation {
            Operation::DatumOnReference { geometry, anchor } => {
                let frame = match &anchor.source {
                    DatumSource::Topology { body_id, reference } => {
                        lookup(body_id).and_then(|solid| topology_frame(solid, reference))
                    }
                    DatumSource::Plane { feature_id } => match placed.get(feature_id.as_str()) {
                        Some(Operation::DatumPlane {
                            origin_mm, normal, ..
                        }) => Some((*origin_mm, *normal)),
                        _ => None,
                    },
                };
                let (origin, direction) = frame
                    .and_then(|(origin, direction)| anchor.place(origin, direction))
                    .ok_or_else(|| NativeError {
                        code: "DATUM_ANCHOR_UNRESOLVED".into(),
                        detail: feature.id.clone(),
                    })?;
                geometry.placed(origin, direction)
            }
            operation if operation.is_datum() => operation.clone(),
            _ => continue,
        };
        if let Some((kind, data, len)) = operation.datum_packet() {
            // SAFETY: the fixed array outlives the synchronous call, and len is 3 or 7.
            if !unsafe { ffi::validate_datum(kind, data.as_ptr(), len) } {
                return Err(NativeError {
                    code: "DATUM_INVALID".into(),
                    detail: feature.id.clone(),
                });
            }
        }
        placed.insert(feature.id.as_str(), operation);
    }
    Ok(placed)
}
