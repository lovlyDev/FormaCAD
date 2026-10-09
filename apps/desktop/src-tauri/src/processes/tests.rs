use super::runner::{bounded_read, stream_read};
use super::*;
use std::{sync::Arc, time::Duration};
use tokio::io::AsyncWriteExt;

#[test]
fn geometry_diagnostic_preserves_feature_identity() {
    let report = r#"{"type":"error","code":"GEOMETRY_BUILD_FAILED","featureId":"Pocket","operation":"boolean","message":"Empty body"}"#;
    let value: serde_json::Value =
        serde_json::from_str(&super::provider_diagnostic(report, "").unwrap()).unwrap();
    assert_eq!(value["featureId"], "Pocket");
    assert_eq!(value["code"], "GEOMETRY_BUILD_FAILED");
}

#[test]
fn saved_cad_python_is_used_without_launch_environment() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("projects");
    std::fs::create_dir(&root).unwrap();
    let executable = std::env::current_exe().unwrap();
    std::fs::write(
        dir.path().join("cad-python.txt"),
        executable.to_str().unwrap(),
    )
    .unwrap();
    assert_eq!(python(&root), Some(executable));
}
#[test]
fn provider_quota_error_survives_exit_failure() {
    let output = r#"{"type":"turn.failed","error":{"message":"You've hit your usage limit. Try again at 8:55 PM."}}"#;
    assert_eq!(
        provider_diagnostic(output, ""),
        Some("You've hit your usage limit. Try again at 8:55 PM.".into())
    );
    assert_eq!(
        provider_diagnostic("", "unauthorized"),
        Some("Authentication is required in your CLI.".into())
    );
    assert!(provider_diagnostic("", "unrecognized failure").is_none());
}
#[tokio::test]
async fn lines_arrive_before_process_output_closes() {
    let (mut writer, reader) = tokio::io::duplex(128);
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let listener: OutputListener = Arc::new(move |line| {
        let _ = tx.send(line.to_owned());
    });
    let task = tokio::spawn(stream_read(reader, Some(listener)));
    writer.write_all(b"first\n").await.unwrap();
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(1), rx.recv())
            .await
            .unwrap()
            .unwrap(),
        "first"
    );
    assert!(!task.is_finished());
    writer.write_all("second".as_bytes()).await.unwrap();
    writer.shutdown().await.unwrap();
    assert_eq!(task.await.unwrap().unwrap(), "first\nsecond");
    assert_eq!(rx.recv().await.unwrap(), "second");
}
#[test]
fn command_injection_is_never_shell_parsed() {
    let exe = std::env::current_exe().unwrap();
    let spec = CommandSpec {
        executable: exe,
        args: vec!["hello; rm -rf /".into(), "$(whoami)".into()],
        cwd: std::env::current_dir().unwrap(),
    };
    let cmd = command(&spec).unwrap();
    let args: Vec<_> = cmd.as_std().get_args().collect();
    assert_eq!(args[0], "hello; rm -rf /");
    assert_eq!(args[1], "$(whoami)");
}
#[tokio::test]
async fn output_limit_is_enforced() {
    let data = vec![b'x'; 2 * 1024 * 1024 + 1];
    assert!(bounded_read(&data[..]).await.is_err());
}
