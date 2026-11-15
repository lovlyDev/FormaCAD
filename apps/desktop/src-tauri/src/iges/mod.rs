//! IGES exchange owns its contract; canonical authored assets remain STEP.
#[cfg(feature = "native-worker-client")]
mod document;
#[cfg(feature = "native-worker-client")]
pub mod export;
#[cfg(feature = "native-worker-client")]
pub mod import;
pub mod schema;
#[cfg(test)]
mod schema_tests;
#[cfg(feature = "native-worker-client")]
mod task;
use crate::core::{AppError, AppState, Result};
use serde::{Deserialize, Serialize};
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportRequest {
    pub project_id: String,
    pub expected_revision: Option<String>,
    pub name: String,
    pub data: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommitRequest {
    pub input: ImportRequest,
    pub source_sha256: String,
    pub prompt: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportRequest {
    pub project_id: String,
    pub expected_revision: String,
    pub body_id: Option<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportOutcome {
    pub path: String,
    pub revision_id: String,
    pub body_id: Option<String>,
    pub report: schema::ExchangeReport,
}
pub(crate) fn failure(code: &str, detail: impl Into<String>) -> AppError {
    AppError::Invalid(serde_json::json!({"code":code,"message":detail.into()}).to_string())
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreview {
    pub report: schema::ExchangeReport,
    pub preview_data: String,
}
#[derive(Serialize)]
pub struct ImportOutcome {
    pub project: crate::models::Project,
    pub report: schema::ExchangeReport,
}
#[tauri::command]
pub async fn preview_iges_import(
    request: ImportRequest,
    state: tauri::State<'_, AppState>,
) -> Result<ImportPreview> {
    #[cfg(feature = "native-worker-client")]
    {
        let executable = crate::native::worker::worker_executable()?;
        import::preview_with_executable(&state, &request, &executable).await
    }
    #[cfg(not(feature = "native-worker-client"))]
    {
        let _ = (request, state);
        Err(failure("IGES_NATIVE_REQUIRED", "Native worker unavailable"))
    }
}
#[tauri::command]
pub async fn import_iges(
    request: CommitRequest,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<ImportOutcome> {
    #[cfg(feature = "native-worker-client")]
    {
        use tauri::Emitter;
        let executable = crate::native::worker::worker_executable()?;
        let result = import::commit_with_executable(&state, &request, &executable).await?;
        let _ = app.emit("project://revision-created", &result.project.id);
        Ok(result)
    }
    #[cfg(not(feature = "native-worker-client"))]
    {
        let _ = (request, state, app);
        Err(failure("IGES_NATIVE_REQUIRED", "Native worker unavailable"))
    }
}
#[tauri::command]
pub async fn export_iges(
    request: ExportRequest,
    locale: Option<String>,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<Option<ExportOutcome>> {
    #[cfg(feature = "native-worker-client")]
    {
        let executable = crate::native::worker::worker_executable()?;
        let output = export::prepare_with_executable(&state, &request, &executable).await?;
        let path = crate::export_destination::save(
            app,
            output.filename,
            "iges".into(),
            locale,
            output.bytes,
        )
        .await?;
        Ok(path.map(|path| ExportOutcome {
            path,
            revision_id: output.revision_id,
            body_id: request.body_id,
            report: output.report,
        }))
    }
    #[cfg(not(feature = "native-worker-client"))]
    {
        let _ = (request, locale, state, app);
        Err(failure("IGES_NATIVE_REQUIRED", "Native worker unavailable"))
    }
}
