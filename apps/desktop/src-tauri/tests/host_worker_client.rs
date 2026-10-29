#![cfg(feature = "native-worker-client")]
//! Run with client-only features and an explicitly built real worker.
use forma_core::native::{health, inspection, section, worker};
use sha2::{Digest, Sha256};
use std::{
    path::PathBuf,
    sync::{atomic::AtomicBool, Arc},
};

fn executable() -> PathBuf {
    std::env::var_os("FORMA_TEST_NATIVE_WORKER")
        .map(PathBuf::from)
        .or_else(|| option_env!("CARGO_BIN_EXE_forma-cad-worker").map(PathBuf::from))
        .expect("Build the real worker first and set FORMA_TEST_NATIVE_WORKER")
}
fn cancel() -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(false))
}

#[tokio::test]
async fn real_worker_build_inspection_section_and_import_remain_available() {
    let root = tempfile::tempdir().unwrap();
    let binary = executable();
    assert_eq!(
        health::probe_isolated_with_executable(&binary, cancel())
            .await
            .unwrap()
            .engine,
        "occt"
    );
    assert_eq!(
        health::probe_with_executable(&binary, root.path(), cancel())
            .await
            .unwrap()
            .engine,
        "occt"
    );
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    let document = serde_json::json!({
        "schemaVersion":2,"revisionId":"client_box","parameters":[],
        "features":[
            {"id":"profile","name":"Profile","operation":{"type":"rectangle","width":{"kind":"literal","mm":40.0},"depth":{"kind":"literal","mm":20.0}}},
            {"id":"pad","name":"Pad","operation":{"type":"extrude","sketchId":"profile","distance":{"kind":"literal","mm":10.0}}}
        ],"bodies":[{"id":"body","name":"Box","sourceFeatureId":"pad"}]
    }).to_string();
    assert!(
        worker::build_with_executable(&document, root.path(), cancel(), &binary)
            .await
            .unwrap()
    );
    let bytes = std::fs::read(root.path().join("model.step")).unwrap();
    assert!(root.path().join("preview.glb").is_file());
    std::fs::write(root.path().join("source.step"), &bytes).unwrap();
    let properties = inspection::inspect_with_executable(root.path(), cancel(), &binary)
        .await
        .unwrap();
    assert!((properties.volume_mm3 - 8000.0).abs() < 1e-6);
    for (actual, expected) in properties.bounds_mm.into_iter().zip([40.0, 20.0, 10.0]) {
        assert!((actual - expected).abs() < 1e-6);
    }
    let hash = format!("{:x}", Sha256::digest(&bytes));
    std::fs::create_dir(root.path().join("inputs")).unwrap();
    let input = root.path().join(format!("inputs/step-{hash}.step"));
    std::fs::write(&input, &bytes).unwrap();
    let report = section::section_with_executable(
        root.path(),
        &hash,
        &section::SectionPlane {
            origin_mm: [0.0, 0.0, 5.0],
            normal: [0.0, 0.0, 1.0],
            deflection_mm: 0.01,
        },
        cancel(),
        &binary,
    )
    .await
    .unwrap();
    assert!((report.geometry.total_length_mm - 120.0).abs() < 1e-6);
    assert_eq!(std::fs::read(&input).unwrap(), bytes);
    let imported = tempfile::tempdir().unwrap();
    std::fs::write(imported.path().join("source.step"), &bytes).unwrap();
    worker::import_step_with_executable(imported.path(), cancel(), &binary)
        .await
        .unwrap();
    assert!(imported.path().join("model.step").is_file());
    assert!(imported.path().join("preview.glb").is_file());
    let properties = inspection::inspect_with_executable(imported.path(), cancel(), &binary)
        .await
        .unwrap();
    assert!((properties.volume_mm3 - 8000.0).abs() < 1e-6);
    imported.close().unwrap();
    root.close().unwrap();
}

#[cfg(windows)]
#[tokio::test]
async fn actual_worker_loader_rejection_is_a_host_error_not_a_host_crash() {
    let root = tempfile::tempdir().unwrap();
    let copied = root.path().join("forma-cad-worker.exe");
    std::fs::copy(executable(), &copied).unwrap();
    // The application-directory DLL takes precedence over the valid DLL on PATH.
    std::fs::write(root.path().join("TKernel.dll"), b"not a PE image").unwrap();
    let error = health::probe_with_executable(&copied, root.path(), cancel())
        .await
        .unwrap_err();
    assert!(error.to_string().contains("NATIVE_KERNEL_UNAVAILABLE"));
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 2);
    root.close()
        .expect("failed worker must exit and release all copied files");
}
