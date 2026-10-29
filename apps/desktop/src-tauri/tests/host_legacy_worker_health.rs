#![cfg(all(windows, feature = "native-worker-client"))]
//! Explicit acceptance against an immutable, externally supplied legacy worker.
//! Launch this test executable from an owned fixture project, with TEMP/TMP and
//! FORMA_TEST_HEALTH_TEMP_ROOT set to a dedicated temporary directory. Ordinary
//! CI deliberately does not need to retain an older application release.

use forma_core::{core::AppError, native::health};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    os::windows::fs::MetadataExt,
    path::{Path, PathBuf},
    sync::{atomic::AtomicBool, Arc},
};

fn required_path(name: &str) -> PathBuf {
    PathBuf::from(std::env::var_os(name).unwrap_or_else(|| panic!("Set {name}")))
        .canonicalize()
        .unwrap_or_else(|error| panic!("Cannot resolve {name}: {error}"))
}

fn sha256(path: &Path) -> String {
    format!("{:x}", Sha256::digest(fs::read(path).unwrap()))
}

fn snapshot(root: &Path) -> BTreeMap<PathBuf, String> {
    fn visit(root: &Path, directory: &Path, entries: &mut BTreeMap<PathBuf, String>) {
        for entry in fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            let metadata = fs::symlink_metadata(&path).unwrap();
            assert_eq!(
                metadata.file_attributes() & 0x400,
                0,
                "Acceptance fixtures must not contain reparse points: {}",
                path.display()
            );
            let relative = path.strip_prefix(root).unwrap().to_path_buf();
            if metadata.is_dir() {
                entries.insert(relative, "directory".into());
                visit(root, &path, entries);
            } else {
                assert!(metadata.is_file(), "Unexpected fixture entry");
                entries.insert(relative, format!("{}:{}", metadata.len(), sha256(&path)));
            }
        }
    }
    let mut entries = BTreeMap::new();
    visit(root, root, &mut entries);
    entries
}

fn scratch_directories(root: &Path) -> Vec<PathBuf> {
    fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("forma-worker-health-")
        })
        .collect()
}

fn assert_unavailable(error: AppError) {
    let AppError::Invalid(text) = error else {
        panic!("Expected native availability error");
    };
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(value["code"], "NATIVE_KERNEL_UNAVAILABLE");
}

#[tokio::test]
#[ignore = "requires an immutable legacy worker and a dedicated acceptance harness"]
async fn legacy_worker_cannot_write_into_installation_or_project_during_health_probe() {
    let worker = required_path("FORMA_TEST_LEGACY_NATIVE_WORKER");
    let expected_hash = std::env::var("FORMA_TEST_LEGACY_NATIVE_WORKER_SHA256")
        .expect("Set the independently verified immutable worker SHA-256");
    assert!(
        expected_hash.len() == 64 && expected_hash.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "Expected a SHA-256 hexadecimal digest"
    );
    assert_eq!(sha256(&worker), expected_hash.to_ascii_lowercase());
    let installation = worker.parent().unwrap();
    let temporary_root = required_path("FORMA_TEST_HEALTH_TEMP_ROOT");
    assert_eq!(
        std::env::temp_dir().canonicalize().unwrap(),
        temporary_root,
        "Set TEMP and TMP before launching this test process"
    );
    let project = std::env::current_dir().unwrap().canonicalize().unwrap();
    assert!(
        project.starts_with(&temporary_root) && project != temporary_root,
        "Launch from an owned fixture project below the dedicated temporary root"
    );
    assert!(
        !installation.starts_with(&temporary_root) && !project.starts_with(installation),
        "Keep the immutable packaged worker outside the disposable harness tree"
    );
    let sentinel = required_path("FORMA_TEST_HEALTH_PROJECT_SENTINEL");
    assert_eq!(sentinel.parent().unwrap(), project);
    assert!(sentinel.is_file() && !fs::read(&sentinel).unwrap().is_empty());
    assert!(
        scratch_directories(&temporary_root).is_empty(),
        "Dedicated harness root already contains readiness scratch directories"
    );
    let installation_before = snapshot(installation);
    let project_before = snapshot(&project);

    // Prove this binary loaded and executed the old stdin protocol, rather than
    // merely failing to load a DLL. Its writes are confined to an owned control.
    let control = tempfile::Builder::new()
        .prefix("forma-legacy-health-control-")
        .tempdir_in(&temporary_root)
        .unwrap();
    let control_result =
        health::probe_with_executable(&worker, control.path(), Arc::new(AtomicBool::new(false)))
            .await;
    let response: serde_json::Value = serde_json::from_slice(
        &fs::read(control.path().join("result.json"))
            .expect("Positive control must execute legacy protocol and create result.json"),
    )
    .unwrap();
    assert_eq!(response["status"], "error");
    assert_eq!(response["code"], "INVALID_REQUEST");
    assert_unavailable(control_result.unwrap_err());
    control
        .close()
        .expect("Legacy worker must release its control directory");

    let result =
        health::probe_isolated_with_executable(&worker, Arc::new(AtomicBool::new(false))).await;
    assert_eq!(snapshot(installation), installation_before);
    assert_eq!(snapshot(&project), project_before);
    assert!(
        scratch_directories(&temporary_root).is_empty(),
        "Readiness scratch directory leaked after legacy worker exit"
    );
    assert_unavailable(result.unwrap_err());
}
