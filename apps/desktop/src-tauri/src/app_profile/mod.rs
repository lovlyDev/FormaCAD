//! Configure application-owned storage before any plugin or WebView is created.
use std::{ffi::OsString, path::PathBuf};
use tauri::utils::config::{AppDirectoriesOverride, Config};

/// Validate first: a rejected override must never initialize the default profile.
pub fn configure(config: &mut Config, directory: Option<OsString>) -> Result<(), &'static str> {
    if let Some(directory) = directory {
        let directory = PathBuf::from(directory);
        if !directory.is_absolute() {
            return Err("FORMA_DATA_DIR must be an absolute path");
        }
        config.app.app_directories_override = Some(AppDirectoriesOverride::Root(directory));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
