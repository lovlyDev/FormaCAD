use forma_core::processes::{run, CommandSpec};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
#[test]
#[ignore = "Process lifecycle test helper, only spawned by the tests below"]
fn fixture_wait() {
    std::thread::sleep(Duration::from_secs(30));
}
fn fixture() -> CommandSpec {
    CommandSpec {
        executable: std::env::current_exe().unwrap(),
        args: vec!["--ignored".into(), "--exact".into(), "fixture_wait".into()],
        cwd: std::env::current_dir().unwrap(),
    }
}
#[tokio::test]
async fn cancellation_reaps_the_process() {
    let cancel = Arc::new(AtomicBool::new(false));
    let flag = cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(250)).await;
        flag.store(true, Ordering::Relaxed);
    });
    let start = Instant::now();
    let result = run(&fixture(), "", cancel, Duration::from_secs(10)).await;
    assert!(result.unwrap_err().to_string().contains("cancelled"));
    assert!(start.elapsed() < Duration::from_secs(5));
}
#[tokio::test]
async fn deadline_reaps_the_process() {
    let start = Instant::now();
    let result = run(
        &fixture(),
        "",
        Arc::new(AtomicBool::new(false)),
        Duration::from_millis(250),
    )
    .await;
    assert!(result.unwrap_err().to_string().contains("timed out"));
    assert!(start.elapsed() < Duration::from_secs(5));
}
#[tokio::test]
async fn unavailable_agent_is_an_error() {
    let mut spec = fixture();
    spec.executable = std::env::temp_dir().join("forma-nonexistent-agent-executable.exe");
    assert!(run(
        &spec,
        "",
        Arc::new(AtomicBool::new(false)),
        Duration::from_secs(1)
    )
    .await
    .is_err());
}
