use super::{command, provider_diagnostic, CommandSpec};
use crate::core::{AppError, Result};
#[cfg(windows)]
use std::path::PathBuf;
use std::{
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
pub(super) async fn bounded_read<R: AsyncRead + Unpin>(reader: R) -> Result<String> {
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
pub(super) async fn stream_read<R: AsyncRead + Unpin>(
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
