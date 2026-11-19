//! Typed optional folder route for the existing permission commands.
use crate::{core::Result, project_repository::window_sources::WindowSources};

#[derive(serde::Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Intent {
    Model {
        program: String,
        prompt: String,
        expected_revision: Option<String>,
        review_plan: Option<crate::agents::review_plan::ReviewPlan>,
    },
    Commands {
        commands: Vec<crate::cad_ir::Command>,
        prompt: String,
        expected_revision: String,
    },
    History {
        direction: Direction,
        expected_revision: Option<String>,
    },
}
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Direction {
    Undo,
    Redo,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Receipt {
    pub id: String,
    pub requires_confirmation: bool,
}

pub(super) struct Request {
    pub project_id: String,
    pub source_id: String,
    pub commit_id: String,
    pub intent: Intent,
    pub detail: String,
}
pub(super) struct Source {
    pub project_id: String,
    pub source_id: String,
    pub commit_id: String,
}

#[cfg(any(
    windows,
    all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )
))]
mod host;

pub(super) async fn request(
    pool: &sqlx::SqlitePool,
    sources: &WindowSources,
    window: &str,
    input: Request,
) -> Result<Receipt> {
    #[cfg(any(
        windows,
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        )
    ))]
    {
        host::request(pool, sources, window, input).await
    }
    #[cfg(not(any(
        windows,
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        )
    )))]
    {
        let Request {
            project_id,
            source_id,
            commit_id,
            intent,
            detail,
        } = input;
        let _ = (
            pool, sources, window, project_id, source_id, commit_id, intent, detail,
        );
        Err(crate::core::AppError::Invalid(
            "PROJECT_FOLDER_UNSUPPORTED".into(),
        ))
    }
}

pub(super) async fn resolve(
    pool: &sqlx::SqlitePool,
    sources: &WindowSources,
    window: &str,
    id: &str,
    allow: bool,
    source: Source,
) -> Result<()> {
    #[cfg(any(
        windows,
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        )
    ))]
    {
        host::resolve(pool, sources, window, id, allow, source).await
    }
    #[cfg(not(any(
        windows,
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        )
    )))]
    {
        let Source {
            project_id,
            source_id,
            commit_id,
        } = source;
        let _ = (
            pool, sources, window, id, allow, project_id, source_id, commit_id,
        );
        Err(crate::core::AppError::Invalid(
            "PROJECT_FOLDER_UNSUPPORTED".into(),
        ))
    }
}
