//! App audit/settings are diagnostics/policy; authority stays in original RAM state.
use super::{Intent, Receipt, Request, Source};
use crate::{
    core::{AppError, Result},
    modeling::{apply::ApplyRequest, prepared_request},
    project_history,
    project_repository::window_sources::{
        approval::{ApprovalSource, WriteIntent},
        WindowSources,
    },
};

fn prepare(original: &crate::models::Project, intent: Intent) -> Result<WriteIntent> {
    match intent {
        Intent::Model {
            program,
            prompt,
            expected_revision,
            review_plan,
        } => {
            let mut request =
                ApplyRequest::new(original.id.clone(), program, prompt, expected_revision);
            request.review_plan = review_plan;
            // The existing pure validator enforces request limits, program and revision.
            let _ = prepared_request::prepare(original, &request)?;
            Ok(WriteIntent::PreparedModel(Box::new(request)))
        }
        Intent::Commands {
            commands,
            prompt,
            expected_revision,
        } => {
            let request = crate::modeling::commands::prepare_for_snapshot(
                original,
                commands,
                prompt,
                expected_revision,
            )?;
            let _ = prepared_request::prepare(original, &request)?;
            Ok(WriteIntent::PreparedCommands(Box::new(request)))
        }
        Intent::History {
            direction,
            expected_revision,
        } => {
            if original.current_revision != expected_revision {
                return Err(AppError::Invalid("PROJECT_SNAPSHOT_CHANGED".into()));
            }
            Ok(WriteIntent::History(match direction {
                super::Direction::Undo => project_history::Direction::Undo,
                super::Direction::Redo => project_history::Direction::Redo,
            }))
        }
    }
}

pub(super) async fn request(
    pool: &sqlx::SqlitePool,
    sources: &WindowSources,
    window: &str,
    input: Request,
) -> Result<Receipt> {
    let Request {
        project_id,
        source_id,
        commit_id,
        intent,
        detail,
    } = input;
    // Only the existing server-owned confirmation policy selects automatic approval.
    let required = super::super::settings_for_pool(pool)
        .await?
        .required("modify_project");
    let source = ApprovalSource {
        window,
        project_id: &project_id,
        source_id: &source_id,
        commit_id: &commit_id,
    };
    let id = sources
        .request_prepared_write(source, move |original| prepare(original, intent))
        .await?;
    // Neither paths nor source/access tokens are written to the audit table.
    // An audit failure never reaches the allow transition. Unreturned pending IDs expire.
    sqlx::query("INSERT INTO permissions(id,project_id,action,detail,decision,created_at) VALUES(?,?,?,?,?,?)")
        .bind(&id).bind(&project_id).bind("modify_project").bind(detail)
        .bind(if required {"pending"}else{"automatic"})
        .bind(chrono::Utc::now().to_rfc3339()).execute(pool).await?;
    if !required {
        sources.resolve_write(&id, window, true).await?;
    }
    Ok(Receipt {
        id,
        requires_confirmation: required,
    })
}

pub(super) async fn resolve(
    pool: &sqlx::SqlitePool,
    sources: &WindowSources,
    window: &str,
    id: &str,
    allow: bool,
    source: Source,
) -> Result<()> {
    let source = ApprovalSource {
        window,
        project_id: &source.project_id,
        source_id: &source.source_id,
        commit_id: &source.commit_id,
    };
    // Audit is never used to recover source identity or resurrect a grant.
    sources.check_write_request(id, source)?;
    if !allow {
        // Denial revokes authority even if writing its audit decision fails.
        sources.resolve_write(id, window, false).await?;
    }
    let changed =
        sqlx::query("UPDATE permissions SET decision=? WHERE id=? AND decision='pending'")
            .bind(if allow { "allowed" } else { "denied" })
            .bind(id)
            .execute(pool)
            .await?;
    if changed.rows_affected() != 1 {
        return Err(AppError::Permission(
            "Request expired or was already resolved".into(),
        ));
    }
    if allow {
        sources.resolve_write(id, window, true).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
