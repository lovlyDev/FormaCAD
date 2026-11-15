use super::schema::*;
#[test]
fn exchange_report_refuses_geometry_loss_forged_binding_and_unexpected_preview() {
    let source = Metrics {
        volume_mm3: 8000.,
        area_mm2: 2800.,
        solid_count: 1,
        face_count: 6,
        edge_count: 12,
        bounds_mm: [40., 20., 10.],
    };
    let report = ExchangeReport {
        schema_version: 1,
        request_id: "request".into(),
        direction: Direction::Import,
        source_sha256: "a".repeat(64),
        source_size: 100,
        source: source.clone(),
        round_trip: source,
        output_sha256: "b".repeat(64),
        output_size: 100,
        preview_sha256: Some("c".repeat(64)),
        preview_size: Some(100),
    };
    let valid =
        |r: &ExchangeReport| r.valid_for("request", Direction::Import, &"a".repeat(64), 100);
    assert!(valid(&report));
    let mut changed = report.clone();
    changed.round_trip.volume_mm3 = 7999.;
    assert!(!valid(&changed));
    let mut changed = report.clone();
    changed.round_trip.solid_count = 2;
    assert!(!valid(&changed));
    let mut changed = report.clone();
    changed.round_trip.bounds_mm[0] = f64::NAN;
    assert!(!valid(&changed));
    let mut changed = report.clone();
    changed.source_sha256 = "d".repeat(64);
    assert!(!valid(&changed));
    let mut changed = report.clone();
    changed.preview_size = None;
    assert!(!valid(&changed));
    assert!(!report.valid_for("other", Direction::Import, &"a".repeat(64), 100));
    assert!(!report.valid_for("request", Direction::Export, &"a".repeat(64), 100));
}
