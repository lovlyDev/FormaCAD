use crate::{
    core::{AppError, Result},
    project_repository::window_sources::WindowSources,
};
use tauri::State;

#[tauri::command]
pub async fn export_folder_project_bundle(
    project_id: String,
    source_id: String,
    expected_commit_id: String,
    locale: Option<String>,
    redact_conversation: bool,
    window: tauri::WebviewWindow,
    sources: State<'_, WindowSources>,
) -> Result<Option<String>> {
    #[cfg(any(
        windows,
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        )
    ))]
    {
        use tauri::Manager;
        let operation = sources
            .capture_operation(window.label(), &source_id, &project_id, &expected_commit_id)
            .await?;
        let suggested = super::suggested_bundle_name(&operation.project().name);
        let Some(path) =
            super::pick_bundle_destination(window.app_handle().clone(), suggested, locale).await?
        else {
            return Ok(None);
        };
        export(operation, path.clone(), redact_conversation).await?;
        Ok(Some(path.display().to_string()))
    }
    #[cfg(not(any(
        windows,
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        )
    )))]
    {
        let _ = (
            project_id,
            source_id,
            expected_commit_id,
            locale,
            redact_conversation,
            window,
            sources,
        );
        Err(AppError::Invalid("PROJECT_FOLDER_UNSUPPORTED".into()))
    }
}

#[cfg(any(
    windows,
    all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )
))]
async fn export(
    operation: crate::project_repository::window_sources::operation::FolderOperation,
    path: std::path::PathBuf,
    redact: bool,
) -> Result<()> {
    use super::prepared::BundlePlan;
    use std::collections::HashMap;

    let destination: crate::project_repository::window_sources::operation::BundleDestination =
        operation.bundle_destination(path)?;
    let snapshot = operation.into_read().await?;
    let history = snapshot
        .read_history()
        .await?
        .ok_or_else(|| AppError::Invalid("HISTORY_INVALID_STATE".into()))?;
    let plan = BundlePlan::new(snapshot.project(), &history, redact)?;
    let mut attachments = HashMap::new();
    for (hash, file) in plan.attachments() {
        attachments.insert(hash, snapshot.read_asset(file).await?);
    }
    snapshot.recheck().await?;
    destination.recheck()?;
    plan.write(destination.path(), attachments)
}

#[cfg(all(
    test,
    any(
        windows,
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        )
    )
))]
mod tests;
