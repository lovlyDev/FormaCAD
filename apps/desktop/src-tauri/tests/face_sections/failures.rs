use super::fixture::*;
use forma_core::cad_ir::Document;
use serde_json::json;

#[tokio::test]
async fn transformed_face_centroid_normal_and_whole_step_section_survive_rotation_translation_mirror(
) {
    let mut value = serde_json::to_value(document()).unwrap();
    value["features"].as_array_mut().unwrap().extend([
        json!({"id":"turn","name":"Turn","operation":{"type":"rotate","bodyFeatureId":"pad","axisOriginMm":[0,0,0],"axisDirection":[0,1,0],"angleDeg":31}}),
        json!({"id":"shift","name":"Shift","operation":{"type":"translate","bodyFeatureId":"turn","offsetMm":[3,-4,9]}}),
        json!({"id":"reflect","name":"Mirror","operation":{"type":"mirror","bodyFeatureId":"shift","planeOriginMm":[7,0,0],"planeNormal":[1,0,0]}})
    ]);
    value["bodies"][0]["sourceFeatureId"] = "reflect".into();
    // Cut only this transformed body to give an independent exact perimeter120 oracle.
    value["bodies"].as_array_mut().unwrap().truncate(1);
    let doc: Document = serde_json::from_value(value).unwrap();
    let (dir, binding) = stage(&doc, &[]);
    let mut q = query("box-face:zmax", -5.);
    q.reference.occurrence_path = vec!["turn".into(), "shift".into(), "reflect".into()];
    let report = section(dir.path(), &binding, &q).await;
    let angle = 31_f64.to_radians();
    let expected_origin = [11. - 5. * angle.sin(), -4., 9. + 5. * angle.cos()];
    let expected_normal = [-angle.sin(), 0., angle.cos()];
    for (actual, expected) in report
        .geometry
        .plane
        .origin_mm
        .into_iter()
        .zip(expected_origin)
    {
        assert!((actual - expected).abs() < 1e-8);
    }
    for (actual, expected) in report
        .geometry
        .plane
        .normal
        .into_iter()
        .zip(expected_normal)
    {
        assert!((actual - expected).abs() < 1e-8);
    }
    assert!((report.geometry.total_length_mm - 120.).abs() < 1e-6);
}

#[tokio::test]
async fn curved_foreign_missing_occurrence_and_out_of_bounds_plane_never_publish_report() {
    for case in 0..4 {
        let (dir, mut binding) = staged();
        let mut q = query("box-face:zmax", -5.);
        let expected = match case {
            0 => {
                binding.body_id = "round_body".into();
                q.reference.owner_feature_id = "cylinder".into();
                q.reference.role = "cylinder-face:side".into();
                q.reference.occurrence_path = vec!["move".into()];
                "MEASUREMENT_UNSUPPORTED"
            }
            1 => {
                q.reference.owner_feature_id = "cylinder".into();
                "TOPOLOGY_REFERENCE_UNRESOLVED"
            }
            2 => {
                q.reference.occurrence_path = vec!["move".into()];
                "TOPOLOGY_REFERENCE_UNRESOLVED"
            }
            _ => {
                q.offset_mm = 10000.;
                "INVALID_SECTION_PLANE"
            }
        };
        let error = section_result(dir.path(), &binding, &q).await.unwrap_err();
        assert!(error.to_string().contains(expected), "case {case}: {error}");
        assert!(!dir.path().join("face-section.json").exists());
        assert!(!dir.path().join("model.step").exists());
    }
}

#[tokio::test]
async fn staged_document_or_source_corruption_is_rejected_before_report_output() {
    for file in ["document.json", "source.step"] {
        let (dir, binding) = staged();
        let path = dir.path().join(file);
        let mut bytes = std::fs::read(&path).unwrap();
        bytes[0] ^= 1;
        std::fs::write(path, bytes).unwrap();
        let error = section_result(dir.path(), &binding, &query("box-face:zmax", -5.))
            .await
            .unwrap_err();
        assert!(
            error.to_string().contains("ASSET_CHECKSUM_MISMATCH"),
            "{error}"
        );
        assert!(!dir.path().join("face-section.json").exists());
    }
}

#[tokio::test]
async fn all_import_seals_are_echoed_and_corruption_of_an_unselected_import_fails_closed() {
    let input = tempfile::tempdir().unwrap();
    let path = input.path().join("input.step");
    forma_core::native::Solid::box_solid(4., 4., 4.)
        .unwrap()
        .write_step(&path)
        .unwrap();
    let bytes = std::fs::read(path).unwrap();
    let hash = forma_core::artifacts::digest(&bytes);
    let mut value = serde_json::to_value(document()).unwrap();
    value["features"].as_array_mut().unwrap().push(json!({"id":"input","name":"Imported","operation":{"type":"importStep","assetId":format!("step_{hash}"),"sha256":hash}}));
    let doc: Document = serde_json::from_value(value).unwrap();
    let (dir, binding) = stage(&doc, &[(hash.clone(), bytes.clone())]);
    let report = section(dir.path(), &binding, &query("box-face:zmax", -5.)).await;
    assert_eq!(report.imported_asset_seals, binding.imported_asset_seals);
    assert_eq!(report.imported_asset_seals.len(), 1);
    let previous = std::fs::read(dir.path().join("face-section.json")).unwrap();
    let mut corrupt = bytes;
    corrupt[0] ^= 1;
    std::fs::write(
        dir.path()
            .join(forma_core::native::assets::relative_path(&hash)),
        corrupt,
    )
    .unwrap();
    let error = section_result(dir.path(), &binding, &query("box-face:zmax", -5.))
        .await
        .unwrap_err();
    assert!(
        error.to_string().contains("ASSET_CHECKSUM_MISMATCH"),
        "{error}"
    );
    assert_eq!(
        std::fs::read(dir.path().join("face-section.json")).unwrap(),
        previous
    );
}
