use crate::core::{AppError, AppState, Result};
use tauri::State;
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
    crate::export_destination::save(app, name, ext, locale, bytes).await
}
