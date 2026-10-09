use crate::core::{AppError, Result};
use tauri_plugin_dialog::DialogExt;
pub async fn save(
    app: tauri::AppHandle,
    name: String,
    ext: String,
    locale: Option<String>,
    bytes: Vec<u8>,
) -> Result<Option<String>> {
    let destination = tokio::task::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_file_name(&name)
            .set_title(if locale.as_deref() == Some("en") {
                "Save CAD model"
            } else {
                "Сохранить CAD-модель"
            })
            .add_filter(
                if locale.as_deref() == Some("en") {
                    "CAD model"
                } else {
                    "CAD-модель"
                },
                &[&ext],
            )
            .blocking_save_file()
    })
    .await
    .map_err(|e| AppError::Invalid(e.to_string()))?;
    let Some(destination) = destination else {
        return Ok(None);
    };
    let path = destination
        .into_path()
        .map_err(|_| AppError::Invalid("Only a local export path is supported".into()))?;
    // The native picker grants this one external destination, not directory-wide access.
    if !path.is_absolute()
        || path.parent().is_none_or(|p| !p.is_dir())
        || std::fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_symlink())
    {
        return Err(AppError::Invalid("Unsafe export destination".into()));
    }
    tokio::fs::write(&path, bytes).await?;
    Ok(Some(path.display().to_string()))
}
