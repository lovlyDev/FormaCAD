use super::{changed, WindowSources};
use crate::{
    core::{AppError, Result},
    project_repository::{backend::RepositoryRead, sqlite_folder::api::FolderSources},
};
use std::sync::Arc;

impl WindowSources {
    pub(super) async fn capture_expected(
        &self,
        label: &str,
        key: &str,
        id: &str,
        expected: &str,
    ) -> Result<RepositoryRead<'static>> {
        let source = self.existing(label)?.lookup(key)?;
        let view = source.capture_metadata().await?;
        if view.project().id != id || view.commit_id() != expected {
            return Err(changed());
        }
        RepositoryRead::from_folder_view(source, view).await
    }
    pub(super) fn for_window(&self, label: &str) -> Result<Arc<FolderSources>> {
        let mut scopes = self.scopes.lock().map_err(|_| changed())?;
        if let Some(scope) = scopes.get(label) {
            return Ok(Arc::clone(scope));
        }
        if scopes.len() >= 16 {
            return Err(AppError::Invalid("PROJECT_SOURCE_LIMIT".into()));
        }
        let scope = Arc::new(FolderSources::default());
        scopes.insert(label.to_owned(), Arc::clone(&scope));
        Ok(scope)
    }
    pub(super) fn existing(&self, label: &str) -> Result<Arc<FolderSources>> {
        self.scopes
            .lock()
            .map_err(|_| changed())?
            .get(label)
            .cloned()
            .ok_or_else(changed)
    }
}

#[cfg(test)]
mod tests;
