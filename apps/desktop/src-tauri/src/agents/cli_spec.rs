//! Build isolated, shell-free agent command lines and stage reference images.
use super::custom::CustomAgentConfig;
use crate::{
    core::{AppError, Result},
    processes::{self, CommandSpec},
};
use std::path::Path;

fn codex_args() -> Vec<String> {
    // Fail closed when a CLI cannot honor these isolation flags.
    let mut args: Vec<String> = [
        "exec",
        "--ignore-user-config",
        "--ignore-rules",
        "--ephemeral",
        "--skip-git-repo-check",
        "--sandbox",
        "read-only",
        "--json",
        "-c",
        "approval_policy=\"never\"",
        "-c",
        "web_search=\"disabled\"",
        "-c",
        "mcp_servers={}",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    for feature in [
        "shell_tool",
        "unified_exec",
        "apps",
        "hooks",
        "plugins",
        "remote_plugin",
        "skill_search",
        "skill_mcp_dependency_install",
        "multi_agent",
        "multi_agent_v2",
        "computer_use",
        "browser_use",
        "browser_use_external",
        "image_generation",
        "view_image",
        "workspace_dependencies",
        "code_mode_host",
        "memories",
        "goals",
        "shell_snapshot",
    ] {
        args.push("-c".into());
        args.push(format!("features.{feature}=false"));
    }
    args.push("-".into());
    args
}

fn claude_args() -> Vec<String> {
    [
        "--bare",
        "--print",
        "--tools",
        "",
        "--strict-mcp-config",
        "--mcp-config",
        "{\"mcpServers\":{}}",
        "--output-format",
        "json",
        "--no-session-persistence",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

pub fn prepare(
    agent: &str,
    custom: Option<&CustomAgentConfig>,
    cwd: &Path,
    images: &[(String, Vec<u8>)],
) -> Result<CommandSpec> {
    if agent == "custom" {
        return custom
            .ok_or_else(|| AppError::Invalid("Custom CLI is not configured".into()))?
            .command_spec(cwd);
    }
    let (executable, mut args) = match agent {
        "codex" => (processes::find_codex(), codex_args()),
        "claude" => (processes::find_executable("claude"), claude_args()),
        _ => (None, Vec::new()),
    };
    let executable = executable.ok_or_else(|| AppError::Invalid(
        "The selected agent is not available. Install Codex or Claude Code, sign in, then restart Forma.".into()
    ))?;
    for (index, (extension, bytes)) in images.iter().enumerate() {
        if agent != "codex" {
            return Err(AppError::Invalid(
                "Reference images require OpenAI Codex".into(),
            ));
        }
        let name = format!("reference-{index}.{extension}");
        crate::artifacts::immutable_write(cwd, Path::new(&name), bytes)?;
        let position = args.len() - 1;
        args.insert(position, "--image".into());
        args.insert(position + 1, cwd.join(name).display().to_string());
    }
    Ok(CommandSpec {
        executable,
        args,
        cwd: cwd.to_path_buf(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn codex_planner_has_no_tools_or_web_access() {
        let args = codex_args();
        assert!(args.contains(&"web_search=\"disabled\"".to_string()));
        assert!(args.contains(&"mcp_servers={}".to_string()));
        assert_eq!(args.last().map(String::as_str), Some("-"));
    }
}
