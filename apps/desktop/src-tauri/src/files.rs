use crate::core::{AppError, AppState, Result};
use tauri::State;
use tauri_plugin_dialog::DialogExt;
#[tauri::command]
pub async fn export_mesh(
    project_id: String,
    name: String,
    bytes: Vec<u8>,
    locale: Option<String>,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<Option<String>> {
    crate::projects::get(&state, &project_id).await?;
    crate::security::file_name(&name)?;
    let ext = std::path::Path::new(&name)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    if bytes.is_empty()
        || bytes.len() > 100 * 1024 * 1024
        || !["stl", "obj", "glb", "3mf"].contains(&ext)
    {
        return Err(AppError::Invalid("Invalid mesh export".into()));
    }
    crate::permissions::consume(&state, &project_id, "export_file").await?;
    let ext = ext.to_owned();
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
