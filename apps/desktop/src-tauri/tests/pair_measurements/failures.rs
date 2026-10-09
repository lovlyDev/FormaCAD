use super::fixture::*;
use forma_core::model_pair_measurement::schema::PairMeasurementQuery;
use serde_json::json;

#[tokio::test]
async fn foreign_second_reference_and_missing_reordered_occurrences_fail_without_report() {
    for case in 0..3 {
        let (dir, binding) = stage(&document(false, 40., 10., true), &[]);
        let mut value = serde_json::to_value(pair(
            "minimumDistance",
            "box-face:xmin",
            "box-face:xmax",
            true,
        ))
        .unwrap();
        match case {
            0 => value["second"]["ownerFeatureId"] = json!("foreign"),
            1 => value["second"]["occurrencePath"] = json!([]),
            _ => value["second"]["occurrencePath"] = json!(["shift", "turn", "reflect"]),
        }
        let q: PairMeasurementQuery = serde_json::from_value(value).unwrap();
        let error = measure(dir.path(), &binding, &q).await.unwrap_err();
        assert!(
            error.to_string().contains("TOPOLOGY_REFERENCE_UNRESOLVED"),
            "{error}"
        );
        assert!(!dir.path().join("pair-measurement.json").exists());
        no_geometry(dir.path());
    }
}

#[tokio::test]
async fn curved_face_and_circular_edge_angles_are_explicitly_unsupported() {
    for (kind, a, b) in [
        (
            "faceNormalAngle",
            "cylinder-face:bottom",
            "cylinder-face:side",
        ),
        (
            "edgeAcuteAngle",
            "cylinder-edge:bottom",
            "cylinder-edge:top",
        ),
    ] {
        let (dir, binding) = stage(&document(true, 8., 15., true), &[]);
        let error = measure(dir.path(), &binding, &pair(kind, a, b, true))
            .await
            .unwrap_err();
        assert!(
            error.to_string().contains("MEASUREMENT_UNSUPPORTED"),
            "{error}"
        );
        assert!(!dir.path().join("pair-measurement.json").exists());
        no_geometry(dir.path());
    }
}

#[tokio::test]
async fn staged_source_and_raw_document_corruption_never_produce_a_pair_report() {
    for file in ["document.json", "source.step"] {
        let (dir, binding) = staged();
        let path = dir.path().join(file);
        let mut bytes = std::fs::read(&path).unwrap();
        bytes[0] ^= 1;
        std::fs::write(path, bytes).unwrap();
        let error = measure(
            dir.path(),
            &binding,
            &query("minimumDistance", "box-face:xmin", "box-face:xmax"),
        )
        .await
        .unwrap_err();
        assert!(
            error.to_string().contains("ASSET_CHECKSUM_MISMATCH"),
            "{error}"
        );
        assert!(!dir.path().join("pair-measurement.json").exists());
    }
}

#[tokio::test]
async fn unselected_import_seals_are_echoed_and_corruption_preserves_the_previous_report() {
    let input = tempfile::tempdir().unwrap();
    let path = input.path().join("input.step");
    forma_core::native::Solid::box_solid(4., 4., 4.)
        .unwrap()
        .write_step(&path)
        .unwrap();
    let bytes = std::fs::read(path).unwrap();
    let hash = forma_core::artifacts::digest(&bytes);
    let mut value = serde_json::to_value(document(false, 40., 10., false)).unwrap();
    value["features"].as_array_mut().unwrap().push(json!({"id":"input","name":"Imported","operation":{"type":"importStep","assetId":format!("step_{hash}"),"sha256":hash}}));
    let doc = serde_json::from_value(value).unwrap();
    let (dir, binding) = stage(&doc, &[(hash.clone(), bytes.clone())]);
    let q = query("minimumDistance", "box-face:xmin", "box-face:xmax");
    let report = measure(dir.path(), &binding, &q).await.unwrap();
    assert_eq!(report.imported_asset_seals, binding.imported_asset_seals);
    assert_eq!(report.imported_asset_seals.len(), 1);
    let prior = std::fs::read(dir.path().join("pair-measurement.json")).unwrap();
    let mut corrupt = bytes;
    corrupt[0] ^= 1;
    std::fs::write(
        dir.path()
            .join(forma_core::native::assets::relative_path(&hash)),
        corrupt,
    )
    .unwrap();
    let error = measure(dir.path(), &binding, &q).await.unwrap_err();
    assert!(
        error.to_string().contains("ASSET_CHECKSUM_MISMATCH"),
        "{error}"
    );
    assert_eq!(
        std::fs::read(dir.path().join("pair-measurement.json")).unwrap(),
        prior
    );
    no_geometry(dir.path());
}
