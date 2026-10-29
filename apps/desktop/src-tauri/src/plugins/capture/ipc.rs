//! Tauri commands that save a plugin-requested video. The destination always
//! comes from the native save picker opened here; the UI only holds the ticket.
use super::CaptureError;
use crate::{
    core::{AppError, Result},
    plugins::{error::PluginError, PluginState},
};
use tauri::State;
use tauri_plugin_dialog::DialogExt;

impl From<CaptureError> for AppError {
    fn from(error: CaptureError) -> Self {
        PluginError::host(error.code(), error.detail()).into()
    }
}

/// Ask where to save the video. `None` means the user cancelled; the ticket
/// is then dropped.
#[tauri::command]
pub async fn plugins_capture_pick(
    token: String,
    file_name: String,
    locale: String,
    plugins: State<'_, PluginState>,
    app: tauri::AppHandle,
) -> Result<Option<String>> {
    let extension = plugins.capture.extension(&token)?;
    let en = locale == "en";
    let picked = tokio::task::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_file_name(format!("{file_name}.{extension}"))
            .set_title(if en {
                "Save video"
            } else {
                "Сохранить видео"
            })
            .add_filter(if en { "Video" } else { "Видео" }, &[extension])
            .blocking_save_file()
    })
    .await
    .map_err(|e| AppError::Invalid(e.to_string()))?;
    let Some(picked) = picked else {
        plugins.capture.finish(&token, false)?;
        return Ok(None);
    };
    let path = picked
        .into_path()
        .map_err(|_| AppError::Invalid("Only a local video path is supported".into()))?;
    if !path.is_absolute()
        || path.parent().is_none_or(|p| !p.is_dir())
        || std::fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_symlink())
    {
        return Err(AppError::Invalid("Unsafe video destination".into()));
    }
    plugins.capture.bind(&token, path.clone())?;
    Ok(Some(path.display().to_string()))
}

/// Raw body: encoded bytes at `x-capture-offset` for ticket `x-capture-token`.
#[tauri::command]
pub fn plugins_capture_write(
    request: tauri::ipc::Request<'_>,
    plugins: State<'_, PluginState>,
) -> Result<()> {
    let header = |name: &str| {
        request
            .headers()
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned)
            .ok_or_else(|| AppError::Invalid(format!("Missing {name}")))
    };
    let token = header("x-capture-token")?;
    let offset = header("x-capture-offset")?
        .parse::<u64>()
        .map_err(|_| AppError::Invalid("Invalid x-capture-offset".into()))?;
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else {
        return Err(AppError::Invalid("Video bytes must be sent raw".into()));
    };
    Ok(plugins.capture.write(&token, offset, bytes)?)
}

/// Keep the finished file or remove the partial one.
#[tauri::command]
pub fn plugins_capture_finish(
    token: String,
    keep: bool,
    plugins: State<'_, PluginState>,
) -> Result<Option<String>> {
    Ok(plugins
        .capture
        .finish(&token, keep)?
        .map(|path| path.display().to_string()))
}
