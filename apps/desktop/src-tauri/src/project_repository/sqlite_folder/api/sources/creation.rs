use super::{stale, FolderProject, FolderSources, OpenedFolder};
use crate::{
    core::{AppError, Result},
    models::Project,
    project_repository::sqlite_folder::api::Publication,
};
use std::path::Path;

/// A creation may be durable even when its live opening cannot be returned.
pub enum FolderCreation {
    Opened(Box<OpenedFolder>),
    SavedWithoutOpening { commit_id: String, error: AppError },
    Indeterminate { commit_id: String, error: AppError },
}

impl FolderSources {
    /// Initialize a new project in an explicitly selected empty folder.
    /// No live source or AppState is required to create it.
    pub async fn create_blank(
        &self,
        root: &Path,
        name: String,
        units: String,
        agent: String,
    ) -> Result<FolderCreation> {
        let opening = self.reserve()?;
        let now = chrono::Utc::now().to_rfc3339();
        let project_id = uuid::Uuid::new_v4().to_string();
        let project = Project {
            schema_version: 1,
            id: project_id.clone(),
            name: name.trim().to_owned(),
            units,
            agent,
            pinned: false,
            thumbnail: None,
            thumbnail_revision: None,
            created_at: now.clone(),
            updated_at: now,
            revisions: vec![],
            current_revision: None,
            messages: vec![],
            files: vec![],
            exports: vec![],
        };
        project.validate()?;
        let commit_id = match FolderProject::create(root, project, vec![], None).await? {
            Publication::Committed(saved) => saved.commit_id().to_owned(),
            Publication::RejectedBefore(error) => return Err(error),
            Publication::Indeterminate { commit_id, error } => {
                return Ok(FolderCreation::Indeterminate { commit_id, error });
            }
        };
        let opened = opening
            .open(root, |snapshot| {
                if snapshot.project().id != project_id || snapshot.commit_id() != commit_id {
                    return Err(stale());
                }
                Ok(())
            })
            .await;
        Ok(match opened {
            Ok(source) => FolderCreation::Opened(Box::new(source)),
            Err(error) => FolderCreation::SavedWithoutOpening { commit_id, error },
        })
    }
}
