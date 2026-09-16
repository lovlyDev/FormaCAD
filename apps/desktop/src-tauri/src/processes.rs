use crate::core::{AppError, Result};
use std::{
    path::{Path, PathBuf},
    process::Stdio,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    process::Command,
};
pub struct CommandSpec {
    pub executable: PathBuf,
    pub args: Vec<String>,
    pub cwd: PathBuf,
}
pub fn command(spec: &CommandSpec) -> Result<Command> {
    crate::security::executable(&spec.executable)?;
    if !spec.cwd.is_absolute() || !spec.cwd.is_dir() {
        return Err(AppError::Invalid(
            "Invalid process working directory".into(),
        ));
    }
    let mut cmd = Command::new(&spec.executable);
    cmd.args(&spec.args)
        .current_dir(&spec.cwd)
        .kill_on_drop(true)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // Drop inherited injection variables; authentication itself remains CLI-owned.
    for key in [
        "NODE_OPTIONS",
        "PYTHONPATH",
        "PYTHONSTARTUP",
        "LD_PRELOAD",
        "DYLD_INSERT_LIBRARIES",
        "CODEX_THREAD_ID",
        "CODEX_SANDBOX_NETWORK_DISABLED",
        "CLAUDECODE",
    ] {
        cmd.env_remove(key);
    }
    #[cfg(windows)]
    cmd.creation_flags(0x08000000);
    #[cfg(unix)]
    cmd.process_group(0);
    Ok(cmd)
}
async fn bounded_read<R: AsyncRead + Unpin>(reader: R) -> Result<String> {
    let mut bytes = Vec::new();
    reader
        .take(2 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .await?;
    if bytes.len() > 2 * 1024 * 1024 {
        return Err(AppError::Invalid(
            "Agent output exceeded the safety limit".into(),
        ));
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}
pub type OutputListener = Arc<dyn Fn(&str) + Send + Sync>;
async fn stream_read<R: AsyncRead + Unpin>(
    mut reader: R,
    listener: Option<OutputListener>,
) -> Result<String> {
    let mut all = Vec::new();
    let mut pending = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let size = reader.read(&mut chunk).await?;
        if size == 0 {
            break;
        }
        if all.len() + size > 2 * 1024 * 1024 {
            return Err(AppError::Invalid(
                "Agent output exceeded the safety limit".into(),
            ));
        }
        all.extend_from_slice(&chunk[..size]);
        pending.extend_from_slice(&chunk[..size]);
        while let Some(end) = pending.iter().position(|b| *b == b'\n') {
            let line: Vec<_> = pending.drain(..=end).collect();
            if let Some(callback) = &listener {
                callback(String::from_utf8_lossy(&line).trim());
            }
        }
    }
    if !pending.is_empty() {
        if let Some(callback) = listener {
            callback(&String::from_utf8_lossy(&pending));
        }
    }
    Ok(String::from_utf8_lossy(&all).into_owned())
}
pub async fn run(
    spec: &CommandSpec,
    input: &str,
    cancel: Arc<AtomicBool>,
    timeout: Duration,
) -> Result<String> {
    run_stream(spec, input, cancel, timeout, None).await
}
pub async fn run_stream(
    spec: &CommandSpec,
    input: &str,
    cancel: Arc<AtomicBool>,
    timeout: Duration,
    listener: Option<OutputListener>,
) -> Result<String> {
    let mut child = command(spec)?.spawn()?;
    #[cfg(windows)]
    let _job = match crate::job::ProcessJob::attach(&child) {
        Ok(job) => job,
        Err(error) => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            return Err(error);
        }
    };
    let pid = child.id();
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| AppError::Invalid("Process input is unavailable".into()))?;
    let text = input.to_owned();
    let input_task = tokio::spawn(async move {
        let mut stdin = stdin;
        stdin.write_all(text.as_bytes()).await?;
        stdin.shutdown().await
    });
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| AppError::Invalid("Process output is unavailable".into()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| AppError::Invalid("Process diagnostics are unavailable".into()))?;
    let mut out_task = tokio::spawn(stream_read(stdout, listener));
    let mut err_task = tokio::spawn(bounded_read(stderr));
    let started = std::time::Instant::now();
    let status = loop {
        if cancel.load(Ordering::Relaxed) || started.elapsed() > timeout {
            terminate_tree(pid).await;
            let _ = child.kill().await;
            let _ = child.wait().await;
            input_task.abort();
            out_task.abort();
            err_task.abort();
            return Err(AppError::Invalid(
                if cancel.load(Ordering::Relaxed) {
                    "Task cancelled. Your last saved revision is unchanged."
                } else {
                    "Agent timed out. Your last saved revision is unchanged."
                }
                .into(),
            ));
        }
        if let Some(status) = child.try_wait()? {
            break status;
        }
        tokio::time::sleep(Duration::from_millis(80)).await;
    };
    let _ = input_task.await;
    let output = tokio::time::timeout(Duration::from_secs(5), &mut out_task).await;
    let errors = tokio::time::timeout(Duration::from_secs(5), &mut err_task).await;
    if output.is_err() || errors.is_err() {
        terminate_tree(pid).await;
        out_task.abort();
        err_task.abort();
        return Err(AppError::Invalid(
            "Agent child processes did not close their output streams".into(),
        ));
    }
    let output = output
        .map_err(|_| AppError::Invalid("Output timed out".into()))?
        .map_err(|e| AppError::Invalid(e.to_string()))??;
    let errors = errors
        .map_err(|_| AppError::Invalid("Diagnostics timed out".into()))?
        .map_err(|e| AppError::Invalid(e.to_string()))??;
    if !status.success() {
        tracing::warn!(code=?status.code(),"Child process failed");
        return Err(AppError::Invalid(
            provider_diagnostic(&output, &errors).unwrap_or_else(|| {
                format!(
                    "Process exited with status {}.",
                    status.code().unwrap_or(-1)
                )
            }),
        ));
    }
    Ok(output)
}
pub fn provider_diagnostic(output: &str, errors: &str) -> Option<String> {
    let mut message = None;
    for line in output.lines() {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
            if v["type"] == "error" && v["code"] == "GEOMETRY_BUILD_FAILED" {
                // Preserve the worker's feature identity through the existing string IPC error.
                return Some(serde_json::json!({
                    "code": v["code"], "featureId": v["featureId"],
                    "operation": v["operation"],
                    "message": v["message"].as_str().unwrap_or("CAD build failed").chars().take(1500).collect::<String>()
                }).to_string());
            }
            if v["type"] == "error" || v["type"] == "turn.failed" || v["is_error"] == true {
                if let Some(text) = v["message"]
                    .as_str()
                    .or(v["error"]["message"].as_str())
                    .or(v["result"].as_str())
                {
                    if text.to_lowercase().contains("usage limit")
                        || text.to_lowercase().contains("quota")
                    {
                        return Some(text.chars().take(4000).collect());
                    }
                    message = Some(text.chars().take(4000).collect::<String>());
                }
            }
        }
    }
    if message.is_some() {
        return message;
    }
    for line in errors.lines().chain(output.lines()) {
        let lower = line.to_lowercase();
        if lower.contains("usage limit") || lower.contains("rate limit") || lower.contains("quota")
        {
            return Some(line.chars().take(4000).collect());
        }
    }
    let diagnostic = safe_diagnostic(errors);
    (!diagnostic.is_empty()).then_some(diagnostic)
}
fn safe_diagnostic(s: &str) -> String {
    let lower = s.to_lowercase();
    if lower.contains("not logged")
        || lower.contains("authentication")
        || lower.contains("unauthorized")
    {
        "Authentication is required in your CLI.".into()
    } else if lower.contains("no module named") {
        "The selected Python environment is missing the CAD kernel (cadquery).".into()
    } else if lower.contains("unexpected argument") || lower.contains("unknown option") {
        "This CLI version does not support the required restricted mode. Update the CLI.".into()
    } else {
        String::new()
    }
}
async fn terminate_tree(pid: Option<u32>) {
    if let Some(pid) = pid {
        #[cfg(windows)]
        {
            if let Some(root) = std::env::var_os("SystemRoot") {
                let mut cmd = Command::new(PathBuf::from(root).join("System32/taskkill.exe"));
                cmd.args(["/PID", &pid.to_string(), "/T", "/F"])
                    .creation_flags(0x08000000)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null());
                let _ = cmd.status().await;
            }
        }
        #[cfg(unix)]
        {
            let _ = Command::new("/bin/kill")
                .args(["-KILL", "--", &format!("-{pid}")])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .await;
        }
    }
}
pub fn find_executable(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH").unwrap_or_default();
    let mut directories: Vec<PathBuf> = std::env::split_paths(&path).collect();
    // Finder and Linux desktop launchers do not inherit an interactive shell PATH.
    if cfg!(unix) {
        directories
            .extend(["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin", "/bin"].map(PathBuf::from));
        if let Some(base) = directories::BaseDirs::new() {
            directories.push(base.home_dir().join(".local/bin"));
            directories.push(base.home_dir().join(".cargo/bin"));
        }
    }
    for dir in directories {
        if !dir.is_absolute() {
            continue;
        }
        let p = dir.join(if cfg!(windows) {
            format!("{name}.exe")
        } else {
            name.to_owned()
        });
        if p.is_file() {
            return p.canonicalize().ok();
        }
    }
    None
}
pub fn find_codex() -> Option<PathBuf> {
    if let Some(p) = find_executable("codex") {
        return Some(p);
    }
    let base = directories::BaseDirs::new()?;
    let roots = [
        base.config_dir()
            .join("npm/node_modules/@openai/codex/node_modules/@openai"),
        base.home_dir()
            .join(".local/share/pnpm/global/5/node_modules/@openai/codex/node_modules/@openai"),
    ];
    for root in roots {
        if let Ok(entries) = std::fs::read_dir(root) {
            for entry in entries.flatten() {
                let vendor = entry.path().join("vendor");
                if let Ok(targets) = std::fs::read_dir(vendor) {
                    for target in targets.flatten() {
                        for folder in ["bin", "codex"] {
                            let p = target.path().join(folder).join(if cfg!(windows) {
                                "codex.exe"
                            } else {
                                "codex"
                            });
                            if p.is_file() {
                                return p.canonicalize().ok();
                            }
                        }
                    }
                }
            }
        }
    }
    None
}
pub fn python(root: &Path) -> Option<PathBuf> {
    let configured = root
        .parent()
        .and_then(|parent| std::fs::read_to_string(parent.join("cad-python.txt")).ok())
        .map(|value| PathBuf::from(value.trim()));
    configured
        .filter(|p| p.is_absolute() && p.is_file())
        .or_else(|| {
            std::env::var_os("FORMA_PYTHON")
                .map(PathBuf::from)
                .filter(|p| p.is_absolute() && p.is_file())
        })
        .or_else(|| {
            let p = root.parent()?.join(if cfg!(windows) {
                "cad-env/Scripts/python.exe"
            } else {
                "cad-env/bin/python"
            });
            p.is_file().then_some(p)
        })
        .or_else(|| find_executable("python3"))
        .or_else(|| find_executable("python"))
}
#[cfg(test)]
mod tests {
    #[test]
    fn geometry_diagnostic_preserves_feature_identity() {
        let report = r#"{"type":"error","code":"GEOMETRY_BUILD_FAILED","featureId":"Pocket","operation":"boolean","message":"Empty body"}"#;
        let value: serde_json::Value =
            serde_json::from_str(&super::provider_diagnostic(report, "").unwrap()).unwrap();
        assert_eq!(value["featureId"], "Pocket");
        assert_eq!(value["code"], "GEOMETRY_BUILD_FAILED");
    }
    use super::*;
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
}
