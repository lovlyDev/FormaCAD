use crate::core::{AppError, Result};
use std::{path::PathBuf, process::Stdio};
use tokio::process::Command;

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
