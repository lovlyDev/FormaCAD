use crate::{
    core::{AppState, Result},
    models::Project,
};
/// Copy one pure committed model/history snapshot; never recover or modify its source.
pub async fn copy_for_editing(state: &AppState, project_id: &str) -> Result<Project> {
    let _write = state.writes.lock().await;
    let snapshot =
        crate::project_repository::legacy::capture_with_history(state, project_id).await?;
    let mut project = snapshot.project.clone();
    for file in &mut project.files {
        let bytes = crate::artifacts::read(state, project_id, file)?;
        if bytes.len() as u64 != file.size {
            return Err(crate::core::AppError::Invalid(
                "Attachment size does not match stored content".into(),
            ));
        }
        if file
            .sha256
            .as_ref()
            .is_some_and(|hash| *hash != crate::artifacts::digest(&bytes))
        {
            return Err(crate::core::AppError::Invalid(
                "Attachment checksum mismatch".into(),
            ));
        }
        if file.data.is_none() {
            file.data = Some(crate::artifacts::data_url(&file.name, &bytes));
        }
    }
    crate::project_repository::legacy::recheck(state, &snapshot).await?;
    let history = snapshot
        .history
        .ok_or_else(|| crate::core::AppError::Invalid("HISTORY_INVALID_STATE".into()))?;
    project.id = uuid::Uuid::new_v4().to_string();
    project.created_at = chrono::Utc::now().to_rfc3339();
    project.updated_at = project.created_at.clone();
    let pending = crate::artifacts::normalize(&mut project)?;
    crate::projects::persist_imported_project(state, project, pending, None, Some(history)).await
}
