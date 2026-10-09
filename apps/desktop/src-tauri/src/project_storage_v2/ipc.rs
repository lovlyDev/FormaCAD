//! Explicit reviewed copy import. The external source directory is never written.
use crate::{
    core::{AppError, AppState, Result},
    models::Project,
};
use std::path::Path;
use tauri::State;
use tauri_plugin_dialog::DialogExt;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderInspection {
    path: String,
    project_id: String,
    name: String,
    current_revision: Option<String>,
    revision_count: usize,
    manifest_sha256: String,
    identity_exists: bool,
}

#[tauri::command]
pub async fn inspect_project_folder(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<Option<FolderInspection>> {
    let picked = tokio::task::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("Forma CAD")
            .blocking_pick_folder()
    })
    .await
    .map_err(|error| AppError::Invalid(error.to_string()))?;
    let Some(picked) = picked else {
        return Ok(None);
    };
    let path = picked
        .into_path()
        .map_err(|error| AppError::Invalid(error.to_string()))?;
    let (project, manifest_sha256) = super::inspect_folder(&path)?;
    let identity_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM projects WHERE id=?)")
            .bind(&project.id)
            .fetch_one(&state.pool)
            .await?;
    Ok(Some(FolderInspection {
        path: path.to_string_lossy().into_owned(),
        project_id: project.id,
        name: project.name,
        current_revision: project.current_revision,
        revision_count: project.revisions.len(),
        manifest_sha256,
        identity_exists,
    }))
}

#[tauri::command]
pub async fn import_project_folder(
    path: String,
    expected_manifest_sha256: String,
    save_copy: bool,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<Project> {
    crate::projects::import_folder_checked(
        &state,
        Path::new(&path),
        save_copy,
        Some(&expected_manifest_sha256),
        Some(&app),
    )
    .await
}
