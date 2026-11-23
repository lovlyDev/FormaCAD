//! Existing managed request/resolve behavior; folder requests never enter this map.
use super::Grant;
use crate::core::{AppError, AppState, Result};
use std::time::{Duration, Instant};
pub(super) async fn request(
    project_id: String,
    action: String,
    detail: String,
    state: &AppState,
) -> Result<String> {
    crate::security::valid_id(&project_id)?;
    if ![
        "modify_project",
        "run_agent",
        "export_file",
        "convert_file",
        "install_dependency",
    ]
    .contains(&action.as_str())
        || detail.len() > 80000
    {
        return Err(AppError::Invalid("Unsupported permission request".into()));
    }
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO permissions(id,project_id,action,detail,created_at) VALUES(?,?,?,?,?)",
    )
    .bind(&id)
    .bind(&project_id)
    .bind(&action)
    .bind(&detail)
    .bind(chrono::Utc::now().to_rfc3339())
    .execute(&state.pool)
    .await?;
    let mut grants = state.grants.lock().await;
    grants.retain(|_, g| g.expires > Instant::now());
    grants.insert(
        id.clone(),
        Grant {
            project_id,
            action,
            approved: false,
            expires: Instant::now() + Duration::from_secs(300),
        },
    );
    Ok(id)
}
pub(super) async fn resolve(id: String, allow: bool, state: &AppState) -> Result<()> {
    let mut grants = state.grants.lock().await;
    let g = grants
        .get_mut(&id)
        .ok_or_else(|| AppError::Permission("Request expired or was already used".into()))?;
    if g.approved || g.expires < Instant::now() {
        return Err(AppError::Permission(
            "Request expired or was already resolved".into(),
        ));
    }
    sqlx::query("UPDATE permissions SET decision=? WHERE id=? AND decision='pending'")
        .bind(if allow { "allowed" } else { "denied" })
        .bind(&id)
        .execute(&state.pool)
        .await?;
    if allow {
        g.approved = true;
    } else {
        grants.remove(&id);
    }
    Ok(())
}
