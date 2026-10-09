#![cfg(feature = "native-occt")]
use forma_core::{
    cad_ir::{apply_commands, Command, Document, Operation, TopologyKind, TopologyReference},
    native::{build_body, Solid},
};
use serde_json::{json, Value};

fn model() -> Document {
    serde_json::from_value(json!({
        "schemaVersion":2,"revisionId":"base","parameters":[{"id":"width","name":"Width","valueMm":40}],
        "features":[
            {"id":"profile","name":"Profile","operation":{"type":"rectangle","width":{"kind":"parameter","parameterId":"width"},"depth":{"kind":"literal","mm":20}}},
            {"id":"pad","name":"Pad","operation":{"type":"extrude","sketchId":"profile","distance":{"kind":"literal","mm":10}}},
            {"id":"offset","name":"Offset","operation":{"type":"translate","bodyFeatureId":"pad","offsetMm":[27,-13,9]}},
            {"id":"spin","name":"Spin","operation":{"type":"rotate","bodyFeatureId":"offset","axisOriginMm":[3,5,7],"axisDirection":[1,2,3],"angleDeg":17}},
            {"id":"mirror","name":"Mirror","operation":{"type":"mirror","bodyFeatureId":"spin","planeOriginMm":[4,-7,2],"planeNormal":[1,-2,3]}},
            {"id":"second","name":"Other branch","operation":{"type":"translate","bodyFeatureId":"pad","offsetMm":[-80,40,0]}}
        ],"bodies":[{"id":"body","name":"Body","sourceFeatureId":"mirror"},{"id":"other","name":"Other","sourceFeatureId":"second"}]
    })).unwrap()
}
fn reference() -> TopologyReference {
    TopologyReference {
        schema_version: 1,
        kind: TopologyKind::Edge,
        owner_feature_id: "pad".into(),
        role: "box-edge:x:ymin:zmax".into(),
        occurrence_path: vec!["offset".into(), "spin".into(), "mirror".into()],
    }
}
fn extras(solid: &Solid) -> Value {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("preview.glb");
    solid.write_glb(&path).unwrap();
    glb_extras(&std::fs::read(path).unwrap())
}
fn glb_extras(bytes: &[u8]) -> Value {
    let length = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
    let document: Value = serde_json::from_slice(&bytes[20..20 + length]).unwrap();
    document["meshes"][0]["primitives"][0]["extras"].clone()
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-5 * b.abs().max(1.), "{a} != {b}");
}
fn add_fillet(document: &mut Document, reference: TopologyReference) {
    document.features.push(serde_json::from_value(json!({"id":"round","name":"Referenced round",
        "operation":{"type":"filletReferencedEdge","bodyFeatureId":"mirror","reference":reference,"radius":{"kind":"literal","mm":1.25}}})).unwrap());
    document.bodies[0].source_feature_id = "round".into();
}

#[test]
fn actual_brep_catalog_has_unique_edge_face_owner_and_transform_path() {
    let document = model();
    let catalog = extras(&build_body(&document, "body").unwrap());
    let edges = catalog["formaEdges"].as_array().unwrap();
    let faces = catalog["formaFaceReferences"].as_array().unwrap();
    assert_eq!((edges.len(), faces.len()), (12, 6));
    let mut roles = std::collections::HashSet::new();
    for edge in edges {
        let decoded: TopologyReference =
            serde_json::from_value(edge["topologyRef"].clone()).unwrap();
        assert!(decoded.is_valid());
        assert_eq!(decoded.kind, TopologyKind::Edge);
        assert_eq!(decoded.owner_feature_id, "pad");
        assert_eq!(decoded.occurrence_path, reference().occurrence_path);
        assert!(roles.insert(decoded.role));
    }
    roles.clear();
    for face in faces {
        let decoded: TopologyReference = serde_json::from_value(face.clone()).unwrap();
        assert!(decoded.is_valid());
        assert_eq!(decoded.kind, TopologyKind::Face);
        assert!(roles.insert(decoded.role));
    }
    let other = extras(&build_body(&document, "other").unwrap());
    assert_eq!(
        other["formaEdges"][0]["topologyRef"]["occurrencePath"],
        json!(["second"])
    );
    // Exact owner cap area follows the logical face through rotation and mirror.
    let cap = faces
        .iter()
        .position(|face| face["role"] == "box-face:zmax")
        .unwrap();
    close(catalog["formaFaceAreasMm2"][cap].as_f64().unwrap(), 800.);
}

#[test]
fn referenced_fillet_matches_independent_local_fillet_after_width_and_transform_edits() {
    for width in [40., 80.] {
        for angle in [17., 61.] {
            let mut document = model();
            document.parameters[0].value_mm = width;
            if let Operation::Rotate { angle_deg, .. } = &mut document.features[3].operation {
                *angle_deg = angle;
            }
            add_fillet(&mut document, reference());
            let actual = build_body(&document, "body").unwrap();
            let expected = Solid::box_solid(width, 20., 10.)
                .unwrap()
                .fillet_edge("box-edge:x:ymin:zmax", 1.25)
                .unwrap()
                .translated(27., -13., 9.)
                .unwrap()
                .rotated([3., 5., 7.], [1., 2., 3.], angle)
                .unwrap()
                .mirrored([4., -7., 2.], [1., -2., 3.])
                .unwrap();
            close(
                actual.volume_mm3(),
                width * 200. - width * 1.25f64.powi(2) * (1. - std::f64::consts::PI / 4.),
            );
            close(actual.volume_mm3(), expected.volume_mm3());
            close(actual.surface_area_mm2(), expected.surface_area_mm2());
            for (a, b) in actual.bounds_mm().into_iter().zip(expected.bounds_mm()) {
                close(a, b);
            }
            close(
                build_body(&document, "other").unwrap().volume_mm3(),
                width * 200.,
            );
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("round.step");
            actual.write_step(&path).unwrap();
            close(
                Solid::read_step(&path).unwrap().volume_mm3(),
                expected.volume_mm3(),
            );
            assert!(extras(&actual)["formaEdges"]
                .as_array()
                .unwrap()
                .iter()
                .all(|edge| edge["topologyRef"].is_null()));
        }
    }
}

#[test]
fn independent_branch_reorder_preserves_catalog_but_wrong_occurrence_never_guesses() {
    let mut document = model();
    let before = extras(&build_body(&document, "body").unwrap());
    let second = document.features.remove(5);
    document.features.insert(2, second);
    let after = extras(&build_body(&document, "body").unwrap());
    assert_eq!(before["formaFaceReferences"], after["formaFaceReferences"]);
    assert_eq!(before["formaEdges"], after["formaEdges"]);
    let mut wrong = reference();
    wrong.occurrence_path = vec!["second".into()];
    add_fillet(&mut document, wrong);
    let unchanged = document.clone();
    assert_eq!(
        build_body(&document, "body").err().unwrap().code,
        "TOPOLOGY_REFERENCE_UNRESOLVED"
    );
    assert_eq!(document, unchanged);
    if let Operation::FilletReferencedEdge { reference, .. } =
        &mut document.features.last_mut().unwrap().operation
    {
        reference.occurrence_path = vec!["spin".into(), "offset".into(), "mirror".into()];
    }
    assert_eq!(
        build_body(&document, "body").err().unwrap().code,
        "TOPOLOGY_REFERENCE_UNRESOLVED"
    );
}

#[test]
fn unsupported_boolean_negative_extrusion_and_step_lookalike_have_no_provenance() {
    let mut document = model();
    if let Operation::Extrude { distance, .. } = &mut document.features[1].operation {
        *distance = forma_core::cad_ir::Dimension::Literal { mm: -10. };
    }
    let negative = build_body(&document, "body").unwrap();
    assert!(extras(&negative)["formaFaceReferences"]
        .as_array()
        .unwrap()
        .iter()
        .all(Value::is_null));
    assert_eq!(
        negative
            .fillet_referenced_edge(&reference(), 1.)
            .err()
            .unwrap()
            .code,
        "TOPOLOGY_REFERENCE_UNSUPPORTED"
    );
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("lookalike.step");
    Solid::box_solid(40., 20., 10.)
        .unwrap()
        .write_step(&path)
        .unwrap();
    let imported = Solid::read_step(&path).unwrap();
    assert!(extras(&imported)["formaEdges"]
        .as_array()
        .unwrap()
        .iter()
        .all(|edge| edge["topologyRef"].is_null()));
    assert_eq!(
        imported
            .fillet_referenced_edge(&reference(), 1.)
            .err()
            .unwrap()
            .code,
        "TOPOLOGY_REFERENCE_UNSUPPORTED"
    );
    let seed = Solid::referenced_box(40., 20., 10., "pad").unwrap();
    let hole = Solid::cylinder(2., 10.).unwrap();
    let cut = seed.cut(&hole).unwrap();
    assert_eq!(
        cut.fillet_referenced_edge(&reference(), 1.)
            .err()
            .unwrap()
            .code,
        "TOPOLOGY_REFERENCE_UNSUPPORTED"
    );
}

#[test]
fn reselection_command_stages_exact_ref_and_invalid_batches_preserve_original() {
    let mut document = model();
    add_fillet(&mut document, reference());
    let original = document.clone();
    let mut changed = reference();
    changed.role = "box-edge:z:xmin:ymin".into();
    let staged = apply_commands(
        &document,
        "base",
        "edited",
        &[Command::SetTopologyReference {
            feature_id: "round".into(),
            reference: changed.clone(),
        }],
    )
    .unwrap();
    build_body(&staged, "body").unwrap();
    assert_eq!(document, original);
    assert!(
        matches!(&staged.features.last().unwrap().operation, Operation::FilletReferencedEdge{reference,..} if reference == &changed)
    );
    let mut invalid = reference();
    invalid.kind = TopologyKind::Face;
    invalid.role = "box-face:zmax".into();
    assert!(apply_commands(
        &document,
        "base",
        "rejected",
        &[Command::SetTopologyReference {
            feature_id: "round".into(),
            reference: invalid
        }]
    )
    .is_err());
    assert_eq!(document, original);
}

#[test]
fn reference_schema_rejects_ordinals_unknown_roles_and_overlong_or_repeated_paths() {
    let base = reference();
    assert!(base.is_valid());
    for role in ["edge[3]", "box-edge:x:ymin:zmiddle", "box-face:zmax"] {
        let mut bad = base.clone();
        bad.role = role.into();
        assert!(!bad.is_valid());
    }
    let mut bad = base.clone();
    bad.occurrence_path.push("offset".into());
    assert!(!bad.is_valid());
    bad.occurrence_path = (0..65).map(|i| format!("move_{i}")).collect();
    assert!(!bad.is_valid());
    let mut json = serde_json::to_value(base).unwrap();
    json["ordinal"] = json!(3);
    assert!(serde_json::from_value::<TopologyReference>(json).is_err());
}

#[tokio::test]
async fn isolated_worker_emits_catalog_and_rejects_wrong_branch_without_outputs() {
    let worker = std::path::Path::new(env!("CARGO_BIN_EXE_forma-cad-worker"));
    let directory = tempfile::tempdir().unwrap();
    let model = model();
    forma_core::native::worker::build_with_executable(
        &serde_json::to_string(&model).unwrap(),
        directory.path(),
        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        worker,
    )
    .await
    .unwrap();
    let catalog = glb_extras(&std::fs::read(directory.path().join("preview.glb")).unwrap());
    assert!(catalog["formaEdges"]
        .as_array()
        .unwrap()
        .iter()
        .any(|edge| edge["topologyRef"] == serde_json::to_value(reference()).unwrap()));
    let rejected = tempfile::tempdir().unwrap();
    let mut bad = model.clone();
    let mut wrong = reference();
    wrong.occurrence_path = vec!["second".into()];
    add_fillet(&mut bad, wrong);
    assert!(forma_core::native::worker::build_with_executable(
        &serde_json::to_string(&bad).unwrap(),
        rejected.path(),
        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        worker
    )
    .await
    .is_err());
    assert!(!rejected.path().join("model.step").exists());
    assert!(!rejected.path().join("preview.glb").exists());
}

#[test]
fn occurrence_limit_preserves_geometry_and_atomically_drops_provenance() {
    let mut body = Solid::referenced_box(40., 20., 10., "pad").unwrap();
    for index in 0..64 {
        body = body
            .translated(1., 0., 0.)
            .unwrap()
            .with_topology_occurrence(&format!("move_{index}"))
            .unwrap();
    }
    let catalog = extras(&body);
    assert!(catalog["formaEdges"]
        .as_array()
        .unwrap()
        .iter()
        .all(|edge| edge["topologyRef"]["occurrencePath"]
            .as_array()
            .unwrap()
            .len()
            == 64));
    let before = body.bounds_mm();
    body = body
        .translated(1., 0., 0.)
        .unwrap()
        .with_topology_occurrence("overflow")
        .unwrap();
    close(body.volume_mm3(), 8000.);
    let after = body.bounds_mm();
    for (actual, expected) in after.into_iter().zip(before) {
        close(actual, expected);
    }
    let dropped = extras(&body);
    assert!(dropped["formaEdges"]
        .as_array()
        .unwrap()
        .iter()
        .all(|edge| edge["topologyRef"].is_null()));
    assert!(dropped["formaFaceReferences"]
        .as_array()
        .unwrap()
        .iter()
        .all(Value::is_null));
    assert_eq!(
        body.fillet_referenced_edge(&reference(), 1.)
            .err()
            .unwrap()
            .code,
        "TOPOLOGY_REFERENCE_UNSUPPORTED"
    );
    // Further transforms remain usable and do not recreate guessed provenance.
    let further = body
        .translated(0., 2., 0.)
        .unwrap()
        .with_topology_occurrence("later")
        .unwrap();
    close(further.volume_mm3(), 8000.);
    assert!(extras(&further)["formaFaceReferences"]
        .as_array()
        .unwrap()
        .iter()
        .all(Value::is_null));
}
