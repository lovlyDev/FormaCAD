//! Tauri commands for plugins. File paths always come from native pickers
//! opened here, never from the UI.
use super::{
    catalog,
    error::PluginError,
    journal::Entry,
    runtime::host::Notice,
    service::{InstallPreview, PlanKind, PluginService, PluginSummary, Source},
    PluginState,
};
use crate::{
    core::{AppError, AppState, Result},
    models::Project,
};
use std::sync::Arc;
use tauri::{Emitter, State};
use tauri_plugin_dialog::DialogExt;

const MAX_IMPORT_BYTES: u64 = 64 * 1024 * 1024;

async fn blocking<T: Send + 'static>(
    service: &Arc<PluginService>,
    work: impl FnOnce(&PluginService) -> super::error::Result<T> + Send + 'static,
) -> Result<T> {
    let service = service.clone();
    tokio::task::spawn_blocking(move || work(&service))
        .await
        .map_err(|e| AppError::Invalid(e.to_string()))?
        .map_err(Into::into)
}

#[tauri::command]
pub async fn plugins_list(plugins: State<'_, PluginState>) -> Result<Vec<PluginSummary>> {
    blocking(&plugins.service, |s| s.list()).await
}

#[tauri::command]
pub async fn plugins_install_file(title: String, filter_name: String, plugins: State<'_, PluginState>, app: tauri::AppHandle) -> Result<Option<InstallPreview>> {
    let picked = tokio::task::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title(title)
            .add_filter(filter_name, &["cadplugin"])
            .blocking_pick_file()
    })
    .await
    .map_err(|e| AppError::Invalid(e.to_string()))?;
    let Some(path) = picked.and_then(|file| file.into_path().ok()) else {
        return Ok(None);
    };
    let bytes = read_limited(&path, super::package::MAX_PACKAGE_BYTES as u64)
        .map_err(|_| PluginError::package("PLUGIN_PACKAGE_TOO_LARGE", "Package is larger than 20 MB"))?;
    blocking(&plugins.service, move |s| s.prepare(&bytes, Source::File)).await.map(Some)
}

fn read_limited(path: &std::path::Path, limit: u64) -> std::io::Result<Vec<u8>> {
    use std::io::Read;
    let mut data = Vec::new();
    std::fs::File::open(path)?.take(limit + 1).read_to_end(&mut data)?;
    if data.len() as u64 > limit {
        return Err(std::io::Error::other("too large"));
    }
    Ok(data)
}

/// Catalog link → signed entry → package → preview. Installs nothing.
#[tauri::command]
pub async fn plugins_install_link(url: String, plugins: State<'_, PluginState>) -> Result<InstallPreview> {
    let link = catalog::parse_link(&url)?;
    let (bytes, entry) = catalog::download(&link).await?;
    let page_url = entry.page_url.clone();
    let preview = blocking(&plugins.service, move |s| s.prepare(&bytes, Source::Catalog { page_url })).await?;
    if let Err(error) = catalog::check_preview(&entry, &preview) {
        plugins.service.cancel(&preview.token);
        return Err(error.into());
    }
    Ok(preview)
}

/// A `formacad://` link that started the app (Windows passes it on the
/// command line). Taken once by the UI, which then asks the user.
#[tauri::command]
pub fn plugins_take_pending_link(plugins: State<'_, PluginState>) -> Option<String> {
    plugins.pending_link.lock().unwrap_or_else(|p| p.into_inner()).take()
}

#[tauri::command]
pub async fn plugins_install_confirm(token: String, plugins: State<'_, PluginState>) -> Result<PluginSummary> {
    blocking(&plugins.service, move |s| s.confirm(&token)).await
}

#[tauri::command]
pub fn plugins_install_cancel(token: String, plugins: State<'_, PluginState>) {
    plugins.service.cancel(&token);
}

#[tauri::command]
pub async fn plugins_set_enabled(id: String, enabled: bool, plugins: State<'_, PluginState>) -> Result<PluginSummary> {
    blocking(&plugins.service, move |s| s.set_enabled(&id, enabled)).await
}

#[tauri::command]
pub async fn plugins_set_active(id: String, version: String, accepted: Vec<String>, plugins: State<'_, PluginState>) -> Result<PluginSummary> {
    blocking(&plugins.service, move |s| s.set_active(&id, &version, &accepted)).await
}

#[tauri::command]
pub async fn plugins_set_settings(id: String, values: serde_json::Value, plugins: State<'_, PluginState>) -> Result<PluginSummary> {
    blocking(&plugins.service, move |s| s.set_settings(&id, &values)).await
}

#[tauri::command]
pub async fn plugins_uninstall(id: String, version: Option<String>, plugins: State<'_, PluginState>) -> Result<()> {
    blocking(&plugins.service, move |s| s.uninstall(&id, version.as_deref())).await
}

#[tauri::command]
pub async fn plugins_journal(id: Option<String>, plugins: State<'_, PluginState>) -> Result<Vec<Entry>> {
    let service = plugins.service.clone();
    tokio::task::spawn_blocking(move || service.journal.recent(id.as_deref(), 100))
        .await
        .map_err(|e| AppError::Invalid(e.to_string()))
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunOutcome {
    pub project: Option<Project>,
    pub notices: Vec<Notice>,
    pub message: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunRequest {
    pub project_id: String,
    pub plugin_id: String,
    pub kind: PlanKind,
    pub contribution_id: String,
    pub params: serde_json::Value,
    pub expected_revision: Option<String>,
    pub prompt: String,
    pub locale: String,
    #[serde(default)]
    pub selection: Option<serde_json::Value>,
}

/// Run a feature or command and apply its plan in one transaction. The UI
/// must have obtained the `modify_project` approval first (same as any edit).
#[tauri::command]
pub async fn plugins_run(
    request: RunRequest,
    plugins: State<'_, PluginState>,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<RunOutcome> {
    let (input, project) = super::apply::input(&state, Some(&request.project_id), &request.locale, request.selection).await?;
    let project = project.ok_or_else(|| AppError::Invalid("Project not found".into()))?;
    if input.revision_id != request.expected_revision {
        return Err(AppError::Invalid("Project changed; retry the CAD edit".into()));
    }
    let (plugin_id, kind, contribution, params) = (request.plugin_id, request.kind, request.contribution_id, request.params);
    let plan = blocking(&plugins.service, move |s| s.plan(&plugin_id, kind, &contribution, &params, &input)).await?;
    let (notices, message) = (plan.report.notices.clone(), plan.message.clone());
    if plan.commands.is_empty() {
        return Ok(RunOutcome { project: None, notices, message });
    }
    let updated = super::apply::apply(&state, &project, plan, request.expected_revision, request.prompt).await?;
    let _ = app.emit("project://revision-created", &updated.id);
    Ok(RunOutcome { project: Some(updated), notices, message })
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportRequest {
    pub project_id: String,
    pub plugin_id: String,
    pub importer_id: String,
    pub params: serde_json::Value,
    pub expected_revision: Option<String>,
    pub prompt: String,
    pub locale: String,
    pub title: String,
}

/// Pick one file, hand its bytes to the importer, apply the result.
#[tauri::command]
pub async fn plugins_import(
    request: ImportRequest,
    plugins: State<'_, PluginState>,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<Option<RunOutcome>> {
    let summary = blocking(&plugins.service, {
        let id = request.plugin_id.clone();
        move |s| s.summary(&id)
    })
    .await?;
    let importer = summary
        .manifest
        .contributes
        .importers
        .iter()
        .find(|i| i.id == request.importer_id)
        .cloned()
        .ok_or_else(|| PluginError::host("PLUGIN_CONTRIBUTION_MISSING", request.importer_id.clone()).for_plugin(&request.plugin_id))?;
    let picker = app.clone();
    let title = request.title.clone();
    let picked = tokio::task::spawn_blocking(move || {
        let extensions: Vec<&str> = importer.extensions.iter().map(|e| e.trim_start_matches('.')).collect();
        picker.dialog().file().set_title(title).add_filter(&importer.title, &extensions).blocking_pick_file()
    })
    .await
    .map_err(|e| AppError::Invalid(e.to_string()))?;
    let Some(path) = picked.and_then(|file| file.into_path().ok()) else {
        return Ok(None);
    };
    let data = read_limited(&path, MAX_IMPORT_BYTES).map_err(|_| AppError::Invalid("The file is larger than 64 MB".into()))?;
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file").to_owned();
    let (input, project) = super::apply::input(&state, Some(&request.project_id), &request.locale, None).await?;
    let project = project.ok_or_else(|| AppError::Invalid("Project not found".into()))?;
    if input.revision_id != request.expected_revision {
        return Err(AppError::Invalid("Project changed; retry the CAD edit".into()));
    }
    let (plugin_id, importer_id, params) = (request.plugin_id, request.importer_id, request.params);
    let plan = blocking(&plugins.service, move |s| s.import(&plugin_id, &importer_id, &file_name, &data, &params, &input)).await?;
    let notices = plan.report.notices.clone();
    let updated = super::apply::apply(&state, &project, plan, request.expected_revision, request.prompt).await?;
    let _ = app.emit("project://revision-created", &updated.id);
    Ok(Some(RunOutcome { project: Some(updated), notices, message: None }))
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportOutcome {
    pub path: Option<String>,
    pub notices: Vec<Notice>,
}

/// Run an exporter and write its bytes to the path the user picks.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn plugins_export(
    project_id: String,
    plugin_id: String,
    exporter_id: String,
    params: serde_json::Value,
    file_name: String,
    locale: String,
    plugins: State<'_, PluginState>,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<ExportOutcome> {
    crate::permissions::consume(&state, &project_id, "export_file").await?;
    let (input, _) = super::apply::input(&state, Some(&project_id), &locale, None).await?;
    let summary = blocking(&plugins.service, {
        let id = plugin_id.clone();
        move |s| s.summary(&id)
    })
    .await?;
    let extension = summary
        .manifest
        .contributes
        .exporters
        .iter()
        .find(|e| e.id == exporter_id)
        .and_then(|e| e.extensions.first())
        .map(|e| e.trim_start_matches('.').to_owned())
        .ok_or_else(|| PluginError::host("PLUGIN_CONTRIBUTION_MISSING", exporter_id.clone()).for_plugin(&plugin_id))?;
    let (bytes, report) = blocking(&plugins.service, move |s| s.export(&plugin_id, &exporter_id, &params, &input)).await?;
    let path = crate::export_destination::save(app, file_name, extension, Some(locale), bytes).await?;
    Ok(ExportOutcome { path, notices: report.notices })
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PanelOutcome {
    pub view: serde_json::Value,
    pub notices: Vec<Notice>,
}

#[tauri::command]
pub async fn plugins_panel(
    project_id: Option<String>,
    plugin_id: String,
    panel_id: String,
    locale: String,
    plugins: State<'_, PluginState>,
    state: State<'_, AppState>,
) -> Result<PanelOutcome> {
    let (input, _) = super::apply::input(&state, project_id.as_deref(), &locale, None).await?;
    let (view, report) = blocking(&plugins.service, move |s| s.panel(&plugin_id, &panel_id, &input)).await?;
    Ok(PanelOutcome { view, notices: report.notices })
}
