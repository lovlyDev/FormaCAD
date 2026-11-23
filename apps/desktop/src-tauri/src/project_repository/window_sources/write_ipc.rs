//! Transport bridge for the existing model and history write commands.
use super::WindowSources;
use crate::{
    core::{AppError, AppState, Result},
    models::Project,
};

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FolderWriteRequest {
    source_id: String,
    expected_commit_id: String,
    approval_id: String,
}

/// Selected by the command handler. The renderer cannot change which
/// operation an IPC endpoint is allowed to execute.
#[derive(Clone, Copy)]
pub enum Command {
    Program,
    Commands,
    History(crate::project_history::Direction),
}

/// Managed callers continue to receive the bare Project object.
#[derive(serde::Serialize)]
#[serde(untagged)]
pub enum WriteReply {
    Managed(Box<Project>),
    #[cfg(any(
        windows,
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        )
    ))]
    Folder(super::approval::FolderWriteReply),
}

impl From<Project> for WriteReply {
    fn from(project: Project) -> Self {
        Self::Managed(Box::new(project))
    }
}

impl FolderWriteRequest {
    fn validate(&self, project_id: &str, has_managed_payload: bool) -> Result<()> {
        if has_managed_payload
            || self.source_id.is_empty()
            || self.source_id.len() > 128
            || [project_id, &self.expected_commit_id, &self.approval_id]
                .into_iter()
                .any(|id| crate::security::valid_id(id).is_err())
        {
            return Err(AppError::Invalid("PROJECT_SNAPSHOT_CHANGED".into()));
        }
        Ok(())
    }
}

pub async fn execute(
    sources: &WindowSources,
    state: &AppState,
    window: &str,
    project_id: &str,
    request: FolderWriteRequest,
    command: Command,
    has_managed_payload: bool,
) -> Result<WriteReply> {
    request.validate(project_id, has_managed_payload)?;
    #[cfg(any(
        windows,
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        )
    ))]
    {
        use super::approval::{ApprovalSource, WriteCommand};
        use crate::project_repository::sqlite_folder::api::ModelRuntime;
        let command = match command {
            Command::Program => WriteCommand::Program,
            Command::Commands => WriteCommand::Commands,
            Command::History(direction) => WriteCommand::History(direction),
        };
        let source = ApprovalSource {
            window,
            source_id: &request.source_id,
            project_id,
            commit_id: &request.expected_commit_id,
        };
        sources
            .execute_write(
                &request.approval_id,
                source,
                command,
                ModelRuntime {
                    tasks: &state.tasks,
                    gate: &state.cad_tasks,
                    runtime_root: &state.root,
                    transient_root: &state.root,
                    executable: None,
                },
            )
            .await
            .map(WriteReply::Folder)
    }
    #[cfg(not(any(
        windows,
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        )
    )))]
    {
        let _ = (sources, state, window, command);
        Err(AppError::Invalid("PROJECT_FOLDER_UNSUPPORTED".into()))
    }
}

#[cfg(test)]
mod tests;
