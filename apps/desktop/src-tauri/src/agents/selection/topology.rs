//! Only kernel-generated, sealed topology can authorize viewport ordinals.
#[cfg(any(feature = "native-occt", test))]
use super::SelectionContext;
use crate::core::AppError;
#[cfg(any(feature = "native-occt", test))]
use crate::core::Result;
#[cfg(any(feature = "native-occt", test))]
use serde_json::Value;

pub(super) fn invalid() -> AppError {
    AppError::Invalid("SELECTION_CONTEXT_INVALID".into())
}

#[cfg(any(feature = "native-occt", test))]
pub(super) fn body_topology(bytes: &[u8], body: &str) -> Result<Value> {
    let field = |offset: usize| -> Result<u32> {
        let value: [u8; 4] = bytes
            .get(offset..offset + 4)
            .ok_or_else(invalid)?
            .try_into()
            .map_err(|_| invalid())?;
        Ok(u32::from_le_bytes(value))
    };
    if bytes.len() > crate::artifacts::MAX_FILE_BYTES
        || field(0)? != 0x4654_6c67
        || field(4)? != 2
        || field(8)? as usize != bytes.len()
        || field(16)? != 0x4e4f_534a
    {
        return Err(invalid());
    }
    let end = 20usize
        .checked_add(field(12)? as usize)
        .ok_or_else(invalid)?;
    let doc: Value =
        serde_json::from_slice(bytes.get(20..end).ok_or_else(invalid)?).map_err(|_| invalid())?;
    let matches: Vec<_> = doc["nodes"]
        .as_array()
        .ok_or_else(invalid)?
        .iter()
        .filter(|node| {
            node["extras"]["formaBodyId"].as_str() == Some(body)
                && node["name"].as_str() == Some(body)
        })
        .collect();
    if matches.len() != 1 {
        return Err(invalid());
    }
    let index = matches[0]["mesh"].as_u64().ok_or_else(invalid)? as usize;
    let primitives = doc["meshes"][index]["primitives"]
        .as_array()
        .ok_or_else(invalid)?;
    if primitives.len() != 1 {
        return Err(invalid());
    }
    let extras = &primitives[0]["extras"];
    for key in ["formaFaceTriangleCounts", "formaFaceAreasMm2", "formaEdges"] {
        let entries = extras[key].as_array().ok_or_else(invalid)?;
        if entries.is_empty() || entries.len() > 100_000 {
            return Err(invalid());
        }
    }
    Ok(extras.clone())
}

#[cfg(any(feature = "native-occt", test))]
pub(super) fn verify(selected: &SelectionContext, fresh: &Value, saved: &Value) -> Result<()> {
    selected.validate(Some(&selected.revision_id))?;
    // Exact mapping equality deliberately fails closed on regenerated topology changes.
    for key in ["formaFaceTriangleCounts", "formaFaceAreasMm2"] {
        if fresh[key] != saved[key] {
            return Err(invalid());
        }
    }
    let fresh_edges = fresh["formaEdges"].as_array().ok_or_else(invalid)?;
    let saved_edges = saved["formaEdges"].as_array().ok_or_else(invalid)?;
    if fresh_edges.len() != saved_edges.len() {
        return Err(invalid());
    }
    for (fresh_edge, saved_edge) in fresh_edges.iter().zip(saved_edges) {
        for field in ["lengthMm", "radiusMm", "points", "semanticKey"] {
            if fresh_edge[field] != saved_edge[field] {
                return Err(invalid());
            }
        }
        if saved_edge.get("topologyRef").is_some()
            && fresh_edge["topologyRef"] != saved_edge["topologyRef"]
        {
            return Err(invalid());
        }
    }
    if saved.get("formaFaceReferences").is_some()
        && fresh["formaFaceReferences"] != saved["formaFaceReferences"]
    {
        return Err(invalid());
    }
    if let Some(ordinal) = selected.face_ordinal {
        if fresh["formaFaceTriangleCounts"]
            .get(ordinal - 1)
            .and_then(Value::as_u64)
            .is_none_or(|triangles| triangles == 0)
        {
            return Err(invalid());
        }
    }
    if let Some(ordinal) = selected.edge_ordinal {
        let edge = fresh["formaEdges"].get(ordinal - 1).ok_or_else(invalid)?;
        if edge["lengthMm"]
            .as_f64()
            .is_none_or(|v| !v.is_finite() || v <= 0.0)
            || selected
                .semantic_key
                .as_deref()
                .is_some_and(|key| edge["semanticKey"].as_str() != Some(key))
        {
            return Err(invalid());
        }
    }
    if let Some(reference) = &selected.topology_ref {
        let expected = serde_json::to_value(reference).map_err(|_| invalid())?;
        let actual = match reference.kind {
            crate::cad_ir::TopologyKind::Edge => {
                &fresh["formaEdges"][selected.edge_ordinal.ok_or_else(invalid)? - 1]["topologyRef"]
            }
            crate::cad_ir::TopologyKind::Face => {
                &fresh["formaFaceReferences"][selected.face_ordinal.ok_or_else(invalid)? - 1]
            }
        };
        if actual != &expected {
            return Err(invalid());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn mapping() -> Value {
        json!({"formaFaceTriangleCounts":[2,2],"formaFaceAreasMm2":[40.,20.],
        "formaEdges":[{"lengthMm":10.,"semanticKey":"box-edge:x:ymin:zmin","radiusMm":null,"points":[[0,0,0],[0.01,0,0]]},
        {"lengthMm":20.,"semanticKey":"box-edge:y:xmin:zmin","radiusMm":null,"points":[[0,0,0],[0,0.02,0]]}]})
    }
    fn selected() -> SelectionContext {
        SelectionContext {
            body_id: "body".into(),
            revision_id: "r".into(),
            face_ordinal: None,
            edge_ordinal: Some(1),
            semantic_key: Some("box-edge:x:ymin:zmin".into()),
            topology_ref: None,
        }
    }
    fn glb(nodes: Value) -> Vec<u8> {
        let document = json!({"nodes":nodes,"meshes":[{"primitives":[{"extras":mapping()}]}]});
        let mut json = serde_json::to_vec(&document).unwrap();
        while !json.len().is_multiple_of(4) {
            json.push(b' ');
        }
        let mut bytes = Vec::new();
        for value in [
            0x4654_6c67,
            2,
            (20 + json.len()) as u32,
            json.len() as u32,
            0x4e4f_534a,
        ] {
            bytes.extend(value.to_le_bytes());
        }
        bytes.extend(json);
        bytes
    }
    #[test]
    fn missing_or_duplicate_body_ids_cannot_authorize_ordinals() {
        let node = json!({"name":"body","mesh":0,"extras":{"formaBodyId":"body"}});
        assert!(body_topology(&glb(json!([node])), "body").is_ok());
        assert!(body_topology(&glb(json!([node])), "forged").is_err());
        assert!(body_topology(&glb(json!([node, node])), "body").is_err());
    }
    #[test]
    fn tampered_ordinals_and_semantic_keys_fail_closed() {
        let trusted = mapping();
        let mut pick = selected();
        assert!(verify(&pick, &trusted, &trusted).is_ok());
        pick.edge_ordinal = Some(3);
        assert!(verify(&pick, &trusted, &trusted).is_err());
        pick.edge_ordinal = Some(2); // valid ordinal, but the key belongs to another edge
        assert!(verify(&pick, &trusted, &trusted).is_err());
        pick.semantic_key = None;
        pick.edge_ordinal = None;
        pick.face_ordinal = Some(3);
        assert!(verify(&pick, &trusted, &trusted).is_err());
        pick.face_ordinal = Some(0);
        assert!(verify(&pick, &trusted, &trusted).is_err());
    }
    #[test]
    fn changed_saved_mapping_rejects_reselection_instead_of_guessing() {
        let trusted = mapping();
        let mut saved = trusted.clone();
        saved["formaEdges"].as_array_mut().unwrap().swap(0, 1);
        assert!(verify(&selected(), &trusted, &saved).is_err());
        saved = trusted.clone();
        saved["formaFaceAreasMm2"][0] = json!(41.);
        assert!(verify(&selected(), &trusted, &saved).is_err());
    }
    #[test]
    fn legacy_preview_without_new_reference_metadata_keeps_identical_ordinals_valid() {
        let saved = mapping();
        let mut fresh = saved.clone();
        fresh["formaEdges"][0]["topologyRef"] = json!({"schemaVersion":1,"kind":"edge",
            "ownerFeatureId":"pad","role":"box-edge:x:ymin:zmin","occurrencePath":[]});
        fresh["formaFaceReferences"] = json!([null, null]);
        assert!(verify(&selected(), &fresh, &saved).is_ok());
        let mut saved_with_reference = fresh.clone();
        saved_with_reference["formaEdges"][0]["topologyRef"]["ownerFeatureId"] = json!("tampered");
        assert!(verify(&selected(), &fresh, &saved_with_reference).is_err());
    }
    #[test]
    fn topology_reference_owner_path_and_kind_must_match_the_selected_ordinal() {
        let reference = json!({"schemaVersion":1,"kind":"edge","ownerFeatureId":"pad",
            "role":"box-edge:x:ymin:zmin","occurrencePath":["moved"]});
        let mut trusted = mapping();
        trusted["formaEdges"][0]["topologyRef"] = reference.clone();
        let mut pick = selected();
        pick.topology_ref = Some(serde_json::from_value(reference).unwrap());
        assert!(verify(&pick, &trusted, &trusted).is_ok());
        pick.topology_ref.as_mut().unwrap().owner_feature_id = "forged".into();
        assert!(verify(&pick, &trusted, &trusted).is_err());
        pick.topology_ref.as_mut().unwrap().owner_feature_id = "pad".into();
        pick.topology_ref.as_mut().unwrap().occurrence_path = vec!["another_move".into()];
        assert!(verify(&pick, &trusted, &trusted).is_err());
        pick.topology_ref.as_mut().unwrap().kind = crate::cad_ir::TopologyKind::Face;
        pick.topology_ref.as_mut().unwrap().role = "box-face:xmin".into();
        assert!(verify(&pick, &trusted, &trusted).is_err());
    }
    #[test]
    fn malformed_glb_headers_do_not_become_topology_context() {
        let mut bytes = glb(json!([]));
        bytes[12..16].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(body_topology(&bytes, "body").is_err());
        assert!(body_topology(b"not a GLB", "body").is_err());
    }
}
