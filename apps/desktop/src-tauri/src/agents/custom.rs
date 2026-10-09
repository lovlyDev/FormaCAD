//! Locally configured agent executable. Arguments are never passed through a shell.
use crate::{
    core::{AppError, AppState, Result},
    processes::CommandSpec,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::State;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CustomAgentConfig {
    pub executable: PathBuf,
    pub args: Vec<String>,
}

impl CustomAgentConfig {
    fn validate(&self) -> Result<()> {
        crate::security::executable(&self.executable)?;
        if self.args.len() > 32
            || self.args.iter().any(|arg| {
                arg.len() > 1024 || arg.chars().any(|ch| ch == '\0' || ch == '\r' || ch == '\n')
            })
        {
            return Err(AppError::Invalid("Invalid Custom CLI arguments".into()));
        }
        Ok(())
    }

    pub fn command_spec(&self, cwd: &Path) -> Result<CommandSpec> {
        self.validate()?;
        Ok(CommandSpec {
            executable: self.executable.clone(),
            args: self.args.clone(),
            cwd: cwd.to_path_buf(),
        })
    }
}

pub async fn load(pool: &sqlx::SqlitePool) -> Result<Option<CustomAgentConfig>> {
    let raw: Option<String> =
        sqlx::query_scalar("SELECT value FROM settings WHERE key='custom_agent'")
            .fetch_optional(pool)
            .await?;
    raw.map(|value| serde_json::from_str(&value).map_err(Into::into))
        .transpose()
}

#[tauri::command]
pub async fn get_custom_agent_config(
    state: State<'_, AppState>,
) -> Result<Option<CustomAgentConfig>> {
    load(&state.pool).await
}

#[tauri::command]
pub async fn set_custom_agent_config(
    config: Option<CustomAgentConfig>,
    state: State<'_, AppState>,
) -> Result<()> {
    if let Some(config) = config {
        config.validate()?;
        sqlx::query("INSERT INTO settings(key,value) VALUES('custom_agent',?) ON CONFLICT(key) DO UPDATE SET value=excluded.value")
            .bind(serde_json::to_string(&config)?)
            .execute(&state.pool)
            .await?;
    } else {
        sqlx::query("DELETE FROM settings WHERE key='custom_agent'")
            .execute(&state.pool)
            .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_arguments_are_structured_and_bounded() {
        let executable = std::env::current_exe().unwrap();
        let config = CustomAgentConfig {
            executable,
            args: vec!["--input".into(), "file name.step".into()],
        };
        assert_eq!(
            config
                .command_spec(std::env::temp_dir().as_path())
                .unwrap()
                .args
                .len(),
            2
        );
        let invalid = CustomAgentConfig {
            args: vec!["line\nbreak".into()],
            ..config
        };
        assert!(invalid.validate().is_err());
    }
}
