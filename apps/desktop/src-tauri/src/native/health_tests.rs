//! Readiness is established by a valid process response, not executable existence.
use super::*;

fn ready() -> String {
    serde_json::json!({
        "protocolVersion": 1, "status": "ready", "engine": "occt",
        "workerVersion": env!("CARGO_PKG_VERSION")
    })
    .to_string()
}

fn assert_unavailable(error: AppError) {
    let AppError::Invalid(text) = error else {
        panic!("Expected native availability error");
    };
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(value["code"], "NATIVE_KERNEL_UNAVAILABLE");
    assert!(!value["message"].as_str().unwrap().is_empty());
}

#[test]
fn readiness_rejects_another_engine_version_and_extra_payload() {
    for patch in [
        serde_json::json!({"protocolVersion": 2}),
        serde_json::json!({"status": "error"}),
        serde_json::json!({"engine": "cadquery"}),
        serde_json::json!({"workerVersion": "0.0.0"}),
        serde_json::json!({"stepSha256": "fake-model"}),
    ] {
        let mut response: serde_json::Value = serde_json::from_str(&ready()).unwrap();
        for (key, value) in patch.as_object().unwrap() {
            response[key] = value.clone();
        }
        assert_unavailable(validate(&response.to_string()).unwrap_err());
    }
    for response in [
        "".into(),
        "not-json".into(),
        " ".repeat(MAX_HEALTH_BYTES + 1),
    ] {
        assert_unavailable(validate(&response).unwrap_err());
    }
}

#[tokio::test]
async fn absent_worker_reports_unavailable_without_creating_artifacts() {
    let directory = tempfile::tempdir().unwrap();
    let missing = directory
        .path()
        .join(format!("missing{}", std::env::consts::EXE_SUFFIX));
    assert_unavailable(
        probe_with_executable(&missing, directory.path(), Arc::new(AtomicBool::new(false)))
            .await
            .unwrap_err(),
    );
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn failed_process_and_successful_nonprotocol_executable_are_not_ready() {
    let directory = tempfile::tempdir().unwrap();
    let executable = std::env::current_exe().unwrap();
    // The actual test executable is an allowed binary. Its invalid argument produces
    // a genuine nonzero exit; --help succeeds but cannot impersonate the worker.
    for argument in ["--forma-invalid-health-fixture-argument", "--help"] {
        assert_unavailable(
            probe_command(
                &CommandSpec {
                    executable: executable.clone(),
                    args: vec![argument.into()],
                    cwd: directory.path().to_path_buf(),
                },
                Arc::new(AtomicBool::new(false)),
                Duration::from_secs(8),
            )
            .await
            .unwrap_err(),
        );
    }
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
}

#[test]
fn compatible_readiness_response_is_accepted() {
    let response = validate(&ready()).unwrap();
    assert_eq!(response.worker_version, env!("CARGO_PKG_VERSION"));
    assert_eq!(response.engine, "occt");
}

#[tokio::test]
async fn cancelled_and_timed_out_readiness_do_not_create_artifacts() {
    let directory = tempfile::tempdir().unwrap();
    let command = CommandSpec {
        executable: std::env::current_exe().unwrap(),
        args: vec!["--help".into()],
        cwd: directory.path().to_path_buf(),
    };
    for (cancel, limit, expected) in [
        (true, Duration::from_secs(8), "cancelled"),
        (false, Duration::ZERO, "timed out"),
    ] {
        let error = probe_command(&command, Arc::new(AtomicBool::new(cancel)), limit)
            .await
            .unwrap_err();
        assert!(error.to_string().contains(expected), "{error}");
        assert_unavailable(error);
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
    }
}
