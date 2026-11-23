use super::{Result, State, WindowSources};

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitObservation {
    project_id: String,
    source_id: String,
    commit_id: String,
    status: CommitStatus,
}

#[cfg_attr(
    not(any(
        windows,
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        )
    )),
    allow(dead_code)
)]
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
enum CommitStatus {
    Current,
    Historical,
    Absent,
}

#[tauri::command]
pub async fn observe_folder_commit(
    project_id: String,
    source_id: String,
    commit_id: String,
    window: tauri::WebviewWindow,
    sources: State<'_, WindowSources>,
) -> Result<CommitObservation> {
    #[cfg(any(
        windows,
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        )
    ))]
    {
        use crate::project_repository::sqlite_folder::api::CommitPresence;
        let status = match sources
            .observe_folder_commit(window.label(), &source_id, &project_id, &commit_id)
            .await?
        {
            CommitPresence::Present { is_head: true } => CommitStatus::Current,
            CommitPresence::Present { is_head: false } => CommitStatus::Historical,
            CommitPresence::Absent => CommitStatus::Absent,
        };
        Ok(CommitObservation {
            project_id,
            source_id,
            commit_id,
            status,
        })
    }
    #[cfg(not(any(
        windows,
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        )
    )))]
    {
        let _ = (project_id, source_id, commit_id, window, sources);
        Err(crate::core::AppError::Invalid(
            "PROJECT_FOLDER_UNSUPPORTED".into(),
        ))
    }
}
