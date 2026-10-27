//! Plugin errors keep "the plugin failed" apart from "CAD failed".
use crate::core::AppError;

/// Where an error came from. Shown differently: a `Plugin` error is the
/// plugin author's problem, a `Package` error means installation was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Origin {
    Package,
    Plugin,
    Host,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{code}: {message}")]
pub struct PluginError {
    /// Stable SCREAMING_SNAKE_CASE code, translated by the UI (`PLUGIN_*`).
    pub code: String,
    /// Technical detail in English; never translated.
    pub message: String,
    pub origin: Origin,
    /// Plugin ID when known.
    pub plugin_id: Option<String>,
}

impl PluginError {
    pub fn package(code: &str, message: impl Into<String>) -> Self {
        Self::new(code, message, Origin::Package)
    }
    pub fn plugin(code: &str, message: impl Into<String>) -> Self {
        Self::new(code, message, Origin::Plugin)
    }
    pub fn host(code: &str, message: impl Into<String>) -> Self {
        Self::new(code, message, Origin::Host)
    }
    fn new(code: &str, message: impl Into<String>, origin: Origin) -> Self {
        Self {
            code: code.to_owned(),
            message: message.into(),
            origin,
            plugin_id: None,
        }
    }
    pub fn for_plugin(mut self, id: &str) -> Self {
        self.plugin_id.get_or_insert_with(|| id.to_owned());
        self
    }
}

impl From<std::io::Error> for PluginError {
    fn from(error: std::io::Error) -> Self {
        Self::host("PLUGIN_IO", error.to_string())
    }
}

impl From<serde_json::Error> for PluginError {
    fn from(error: serde_json::Error) -> Self {
        Self::host("PLUGIN_DATA_INVALID", error.to_string())
    }
}

impl From<AppError> for PluginError {
    fn from(error: AppError) -> Self {
        Self::host("PLUGIN_HOST", error.to_string())
    }
}

/// IPC boundary: the UI renders `{targetId, code, message}` through `errorText`.
impl From<PluginError> for AppError {
    fn from(error: PluginError) -> Self {
        AppError::Invalid(
            serde_json::json!({
                "code": error.code,
                "message": error.message,
                "targetId": error.plugin_id,
                "origin": error.origin,
            })
            .to_string(),
        )
    }
}

pub type Result<T> = std::result::Result<T, PluginError>;
