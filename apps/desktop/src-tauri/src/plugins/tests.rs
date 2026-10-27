//! End-to-end plugin tests: the example `.cadplugin` from the repository, plus
//! hand-built components for traps, limits and forbidden imports.
use super::{
    package::tests::zip,
    runtime::Limits,
    service::{CallInput, PlanKind, PluginService, Source},
};
use crate::cad_ir::{apply_commands, Document, SCHEMA_VERSION};
use serde_json::{json, Value};
use std::{io::Read, time::Duration};

const EXAMPLE: &[u8] = include_bytes!(
    "../../../../../plugins/examples/simple-gear/prebuilt/simple-gear-0.1.0.cadplugin"
);

fn service() -> (tempfile::TempDir, PluginService) {
    let dir = tempfile::tempdir().unwrap();
    let service = PluginService::new(dir.path().join("plugins"));
    (dir, service)
}

fn files(bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    (0..archive.len())
        .map(|i| {
            let mut file = archive.by_index(i).unwrap();
            let mut data = vec![];
            file.read_to_end(&mut data).unwrap();
            (file.name().unwrap().into_owned(), data)
        })
        .collect()
}

/// Same package with an edited manifest (and optionally another wasm).
fn repack(bytes: &[u8], edit: impl FnOnce(&mut Value), wasm: Option<&[u8]>) -> Vec<u8> {
    let mut entries = files(bytes);
    let mut edit = Some(edit);
    for (name, data) in entries.iter_mut() {
        if name == "manifest.json" {
            let mut manifest: Value = serde_json::from_slice(data).unwrap();
            (edit.take().unwrap())(&mut manifest);
            *data = manifest.to_string().into_bytes();
        } else if name == "plugin.wasm" {
            if let Some(wasm) = wasm {
                *data = wasm.to_vec();
            }
        }
    }
    let borrowed: Vec<(&str, &[u8])> = entries
        .iter()
        .map(|(n, d)| (n.as_str(), d.as_slice()))
        .collect();
    zip(&borrowed)
}

fn install(service: &PluginService, bytes: &[u8]) {
    let preview = service.prepare(bytes, Source::File).unwrap();
    service.confirm(&preview.token).unwrap();
}

fn empty() -> Document {
    Document {
        schema_version: SCHEMA_VERSION,
        revision_id: "base".into(),
        parameters: vec![],
        features: vec![],
        bodies: vec![],
    }
}

fn input(locale: &str) -> CallInput {
    CallInput {
        locale: locale.into(),
        ..Default::default()
    }
}

#[test]
fn example_installs_builds_an_operation_and_uninstalls() {
    let (_dir, service) = service();
    let preview = service.prepare(EXAMPLE, Source::File).unwrap();
    assert_eq!(preview.manifest.id, "simple-gear");
    assert_eq!(preview.permissions, vec!["model.write", "notifications"]);
    assert_eq!(preview.added_permissions, preview.permissions);
    assert!(!preview.is_update && !preview.already_installed);
    // Nothing is installed before confirmation.
    assert!(service.list().unwrap().is_empty());
    let summary = service.confirm(&preview.token).unwrap();
    assert!(summary.enabled);
    assert_eq!(summary.translations["ru"]["name"], "Простая шестерня");
    assert!(service
        .store
        .version_dir("simple-gear", "0.1.0")
        .join("plugin.wasm")
        .is_file());

    let plan = service
        .plan(
            "simple-gear",
            PlanKind::Feature,
            "simpleGear",
            &json!({"teeth": 8, "bore": 6}),
            &input("ru"),
        )
        .unwrap();
    // 5 setup features + 2 per tooth + bore + body.
    assert_eq!(plan.commands.len(), 5 + 16 + 1 + 1);
    assert_eq!(plan.report.notices.len(), 1);
    assert!(plan.report.notices[0].message.contains('8'));
    assert!(plan.report.fuel_used > 0);
    let document = apply_commands(&empty(), "base", "next", &plan.commands).unwrap();
    assert_eq!(document.bodies.len(), 1);
    assert_eq!(document.bodies[0].name, "Шестерня");
    assert!(document.features.iter().all(|f| f.id.starts_with('p')));

    // Out-of-range parameters are refused by the host before the plugin runs.
    let error = service
        .plan(
            "simple-gear",
            PlanKind::Feature,
            "simpleGear",
            &json!({"teeth": 4}),
            &input("en"),
        )
        .unwrap_err();
    assert_eq!(error.code, "PLUGIN_PARAMS_INVALID");
    // Plugin-side validation is reported as a plugin error.
    let error = service
        .plan(
            "simple-gear",
            PlanKind::Feature,
            "simpleGear",
            &json!({"bore": 80}),
            &input("en"),
        )
        .unwrap_err();
    assert_eq!(
        (error.code.as_str(), error.origin),
        ("BORE_TOO_LARGE", super::error::Origin::Plugin)
    );

    service.set_enabled("simple-gear", false).unwrap();
    let error = service
        .plan(
            "simple-gear",
            PlanKind::Feature,
            "simpleGear",
            &json!({}),
            &input("en"),
        )
        .unwrap_err();
    assert_eq!(error.code, "PLUGIN_DISABLED");

    service.uninstall("simple-gear", None).unwrap();
    assert!(service.list().unwrap().is_empty());
    assert!(!service.store.root.join("simple-gear").exists());
    let events: Vec<_> = service
        .journal
        .recent(Some("simple-gear"), 20)
        .into_iter()
        .map(|e| e.event)
        .collect();
    assert!(
        events.contains(&"install".to_owned())
            && events.contains(&"call".to_owned())
            && events[0] == "uninstall"
    );
}

#[test]
fn import_without_permission_is_rejected_at_install() {
    let (_dir, service) = service();
    let bytes = repack(EXAMPLE, |m| m["permissions"] = json!(["model.write"]), None);
    let error = service.prepare(&bytes, Source::File).unwrap_err();
    assert_eq!(error.code, "PLUGIN_IMPORT_NOT_PERMITTED");
    assert!(error.message.contains("notifications"));
}

#[test]
fn updates_show_new_permissions_and_versions_are_immutable() {
    let (_dir, service) = service();
    install(&service, EXAMPLE);
    let update = repack(
        EXAMPLE,
        |m| {
            m["version"] = "0.2.0".into();
            m["permissions"] = json!(["model.write", "notifications", "storage"]);
        },
        None,
    );
    let preview = service.prepare(&update, Source::File).unwrap();
    assert!(preview.is_update);
    assert_eq!(preview.added_permissions, vec!["storage"]);
    service.cancel(&preview.token);
    assert_eq!(service.summary("simple-gear").unwrap().active, "0.1.0");
    let preview = service.prepare(&update, Source::File).unwrap();
    service.confirm(&preview.token).unwrap();
    let summary = service.summary("simple-gear").unwrap();
    assert_eq!(
        (summary.active.as_str(), summary.versions.len()),
        ("0.2.0", 2)
    );
    // Going back needs no new consent; going forward again does.
    service
        .set_active(
            "simple-gear",
            "0.1.0",
            &["model.write".into(), "notifications".into()],
        )
        .unwrap();
    let error = service
        .set_active(
            "simple-gear",
            "0.2.0",
            &["model.write".into(), "notifications".into()],
        )
        .unwrap_err();
    assert_eq!(error.code, "PLUGIN_CONSENT_REQUIRED");

    let tampered = repack(EXAMPLE, |m| m["description"] = "changed".into(), None);
    assert_eq!(
        service.prepare(&tampered, Source::File).unwrap_err().code,
        "PLUGIN_VERSION_CONFLICT"
    );
    assert_eq!(
        service.confirm("missing-token").unwrap_err().code,
        "PLUGIN_INSTALL_EXPIRED"
    );
}

#[test]
fn broken_archives_and_traversal_never_reach_the_profile() {
    let (_dir, service) = service();
    let mut truncated = EXAMPLE.to_vec();
    truncated.truncate(EXAMPLE.len() - 100);
    assert_eq!(
        service.prepare(&truncated, Source::File).unwrap_err().code,
        "PLUGIN_PACKAGE_CORRUPT"
    );
    let mut entries = files(EXAMPLE);
    entries.push(("assets/../../escape.txt".into(), b"x".to_vec()));
    let borrowed: Vec<(&str, &[u8])> = entries
        .iter()
        .map(|(n, d)| (n.as_str(), d.as_slice()))
        .collect();
    assert_eq!(
        service
            .prepare(&zip(&borrowed), Source::File)
            .unwrap_err()
            .code,
        "PLUGIN_PACKAGE_PATH"
    );
    assert!(
        !service.store.root.exists()
            || std::fs::read_dir(&service.store.root)
                .unwrap()
                .next()
                .is_none()
    );
}

// --- Hand-built components -------------------------------------------------

const WIT: &str = include_str!("../../../../../docs/plugin-api/wit/formacad-plugin.wit");

/// Core module exporting `feature.build` with `body` as its code, encoded
/// as a component against `world` (written in WIT).
fn component(world: &str, extra_packages: &str, imports: &str, body: &str) -> Vec<u8> {
    let mut resolve = wit_parser::Resolve::default();
    resolve.push_str("formacad.wit", WIT).unwrap();
    if !extra_packages.is_empty() {
        resolve.push_str("extra.wit", extra_packages).unwrap();
    }
    let package = resolve.push_str("test.wit", world).unwrap();
    let world = resolve.select_world(&[package], Some("test")).unwrap();
    let mut module = wat::parse_str(format!(
        r#"(module
            {imports}
            (memory (export "memory") 1)
            (func (export "cabi_realloc") (param i32 i32 i32 i32) (result i32) i32.const 1024)
            (func (export "formacad:plugin/feature@0.1.0#build") (param i32 i32 i32 i32 i32 i32) (result i32) {body})
            (func (export "cabi_post_formacad:plugin/feature@0.1.0#build") (param i32)))"#
    ))
    .unwrap();
    wit_component::embed_component_metadata(
        &mut module,
        &resolve,
        world,
        wit_component::StringEncoding::UTF8,
        false,
    )
    .unwrap();
    wit_component::ComponentEncoder::default()
        .module(&module)
        .unwrap()
        .validate(true)
        .encode()
        .unwrap()
}

const FEATURE_WORLD: &str = "package test:plugin; world test { include formacad:plugin/host@0.1.0; export formacad:plugin/feature@0.1.0; }";

fn wat_package(wasm: &[u8]) -> Vec<u8> {
    repack(
        EXAMPLE,
        |m| {
            m["id"] = "wat-plugin".into();
            m["permissions"] = json!(["model.write"]);
            m["contributes"]["features"][0]["params"] = json!([]);
        },
        Some(wasm),
    )
}

fn run(service: &PluginService) -> super::error::PluginError {
    service
        .plan(
            "wat-plugin",
            PlanKind::Feature,
            "simpleGear",
            &json!({}),
            &input("en"),
        )
        .unwrap_err()
}

#[test]
fn endless_loop_runs_out_of_fuel_or_time_without_harming_the_host() {
    let wasm = component(FEATURE_WORLD, "", "", "(loop br 0) i32.const 0");
    let dir = tempfile::tempdir().unwrap();
    let fuel = PluginService::new(dir.path().join("a")).with_limits(Limits {
        fuel: 50_000_000,
        ..Limits::default()
    });
    install(&fuel, &wat_package(&wasm));
    let error = run(&fuel);
    assert_eq!(
        (error.code.as_str(), error.origin),
        ("PLUGIN_FUEL_EXHAUSTED", super::error::Origin::Plugin)
    );
    assert_eq!(error.plugin_id.as_deref(), Some("wat-plugin"));

    let slow = PluginService::new(dir.path().join("b")).with_limits(Limits {
        fuel: u64::MAX / 2,
        timeout: Duration::from_millis(300),
        ..Limits::default()
    });
    install(&slow, &wat_package(&wasm));
    let started = std::time::Instant::now();
    assert_eq!(run(&slow).code, "PLUGIN_TIMEOUT");
    assert!(started.elapsed() < Duration::from_secs(5));
    // The host keeps working after both interruptions.
    assert_eq!(slow.list().unwrap().len(), 1);
}

#[test]
fn memory_beyond_256_mb_is_refused() {
    // 5000 pages = 312.5 MB; the limiter refuses, the plugin then traps.
    let wasm = component(
        FEATURE_WORLD,
        "",
        "",
        "(drop (memory.grow (i32.const 5000))) unreachable",
    );
    let (_dir, service) = service();
    install(&service, &wat_package(&wasm));
    assert_eq!(run(&service).code, "PLUGIN_MEMORY_LIMIT");
    // Growth inside the limit works and an ordinary trap is reported as such.
    let wasm = component(
        FEATURE_WORLD,
        "",
        "",
        "(drop (memory.grow (i32.const 100))) unreachable",
    );
    let (_dir, service) = self::service();
    install(&service, &wat_package(&wasm));
    assert_eq!(run(&service).code, "PLUGIN_TRAP");
}

#[test]
fn wasi_and_foreign_imports_are_forbidden() {
    let wasi =
        "package wasi:cli@0.2.0; interface environment { get-arguments: func() -> list<string>; }";
    let world = "package test:plugin; world test { import wasi:cli/environment@0.2.0; export formacad:plugin/feature@0.1.0; }";
    let wasm = component(
        world,
        wasi,
        r#"(import "wasi:cli/environment@0.2.0" "get-arguments" (func $args (param i32)))"#,
        "(call $args (i32.const 0)) i32.const 0",
    );
    let (_dir, service) = service();
    let error = service
        .prepare(&wat_package(&wasm), Source::File)
        .unwrap_err();
    assert_eq!(error.code, "PLUGIN_IMPORT_FORBIDDEN");
    assert!(error.message.contains("wasi:cli"));
}

#[test]
fn bad_plugin_output_is_a_plugin_error() {
    // Returns ok("[1]"), which is not a command list: result<string> lowers as
    // [tag=0][ptr][len] at the return area.
    let body = r#"(i32.store8 (i32.const 2048) (i32.const 91))
        (i32.store8 (i32.const 2049) (i32.const 49))
        (i32.store8 (i32.const 2050) (i32.const 93))
        (i32.store (i32.const 4096) (i32.const 0))
        (i32.store (i32.const 4100) (i32.const 2048))
        (i32.store (i32.const 4104) (i32.const 3))
        i32.const 4096"#;
    let wasm = component(FEATURE_WORLD, "", "", body);
    let (_dir, service) = service();
    install(&service, &wat_package(&wasm));
    let error = run(&service);
    assert_eq!(
        (error.code.as_str(), error.origin),
        ("PLUGIN_OUTPUT_INVALID", super::error::Origin::Plugin)
    );
}
