#![cfg(feature = "native-occt")]
//! The repository's example plugin, installed from its `.cadplugin`, builds
//! real OCCT geometry through the Command API.
use forma_core::{
    cad_ir::{apply_commands, Document, SCHEMA_VERSION},
    native::build_body,
    plugins::service::{CallInput, PlanKind, PluginService, Source},
};
use serde_json::json;

const EXAMPLE: &[u8] = include_bytes!("../../../../plugins/examples/simple-gear/prebuilt/simple-gear-0.1.0.cadplugin");

fn close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-5 * expected.abs().max(1.), "{actual} != {expected}");
}

#[test]
fn installed_example_plugin_builds_an_analytic_gear() {
    let dir = tempfile::tempdir().unwrap();
    let service = PluginService::new(dir.path().join("plugins"));
    let preview = service.prepare(EXAMPLE, Source::File).unwrap();
    service.confirm(&preview.token).unwrap();

    let (r, l, w, t, bore, n) = (20.0_f64, 4.0_f64, 3.0_f64, 8.0_f64, 8.0_f64, 16.0_f64);
    let plan = service
        .plan(
            "simple-gear",
            PlanKind::Feature,
            "simpleGear",
            &json!({"teeth": 16, "rootRadius": r, "toothLength": l, "toothWidth": w, "thickness": t, "bore": bore}),
            &CallInput { locale: "en".into(), ..Default::default() },
        )
        .unwrap();
    let base = Document { schema_version: SCHEMA_VERSION, revision_id: "base".into(), parameters: vec![], features: vec![], bodies: vec![] };
    let document = apply_commands(&base, "base", "gear", &plan.commands).unwrap();
    let body = document.bodies[0].id.clone();
    let gear = build_body(&document, &body).unwrap();

    // Disc + n × (tooth part outside the root circle) − bore, all × thickness.
    let h = w / 2.0;
    let outside = (r + l) * w - (h * (r * r - h * h).sqrt() + r * r * (h / r).asin());
    let expected = (std::f64::consts::PI * (r * r - (bore / 2.0).powi(2)) + n * outside) * t;
    close(gear.volume_mm3(), expected);
    close(gear.bounds_mm()[0], 2.0 * (r + l));
    close(gear.bounds_mm()[2], t);

    service.uninstall("simple-gear", None).unwrap();
    assert!(service.list().unwrap().is_empty());
}
