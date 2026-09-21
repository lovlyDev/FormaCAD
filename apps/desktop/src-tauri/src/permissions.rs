use crate::core::{AppError, AppState, Result};
use std::time::{Duration, Instant};
use tauri::State;
const ACTIONS: [&str; 5] = [
    "modify_project",
    "run_agent",
    "export_file",
    "convert_file",
    "install_dependency",
];
#[derive(serde::Serialize, serde::Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ConfirmationSettings {
    pub mode: String,
    pub overrides: std::collections::HashMap<String, bool>,
}
impl Default for ConfirmationSettings {
    fn default() -> Self {
        Self {
            mode: "all".into(),
            overrides: Default::default(),
        }
    }
}
impl ConfirmationSettings {
    fn required(&self, action: &str) -> bool {
        self.overrides
            .get(action)
            .copied()
            .unwrap_or(match self.mode.as_str() {
                "none" => false,
                "cli" => action == "run_agent" || action == "install_dependency",
                _ => true,
            })
    }
}
async fn settings(state: &AppState) -> Result<ConfirmationSettings> {
    let raw: Option<String> =
        sqlx::query_scalar("SELECT value FROM settings WHERE key='confirmations'")
            .fetch_optional(&state.pool)
            .await?;
    Ok(match raw {
        Some(raw) => serde_json::from_str(&raw)?,
        None => ConfirmationSettings::default(),
    })
}
#[tauri::command]
pub async fn get_confirmation_settings(state: State<'_, AppState>) -> Result<ConfirmationSettings> {
    settings(&state).await
}
#[tauri::command]
pub async fn set_confirmation_settings(
    value: ConfirmationSettings,
    state: State<'_, AppState>,
) -> Result<()> {
    if !["all", "cli", "none"].contains(&value.mode.as_str())
        || value
            .overrides
            .keys()
            .any(|k| !ACTIONS.contains(&k.as_str()))
    {
        return Err(AppError::Invalid("Invalid confirmation settings".into()));
    }
    sqlx::query("INSERT INTO settings(key,value) VALUES('confirmations',?) ON CONFLICT(key) DO UPDATE SET value=excluded.value").bind(serde_json::to_string(&value)?).execute(&state.pool).await?;
    Ok(())
}
#[derive(Debug)]
pub struct Grant {
    pub project_id: String,
    pub action: String,
    pub approved: bool,
    pub expires: Instant,
}
#[derive(serde::Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct AuditEntry {
    id: String,
    project_id: String,
    action: String,
    detail: String,
    decision: String,
    created_at: String,
}
#[tauri::command]
pub async fn permission_audit(state: State<'_, AppState>) -> Result<Vec<AuditEntry>> {
    Ok(sqlx::query_as::<_,AuditEntry>("SELECT id,project_id,action,detail,decision,created_at FROM permissions ORDER BY created_at DESC LIMIT 100").fetch_all(&state.pool).await?)
}
#[tauri::command]
pub async fn request_permission(
    project_id: String,
    action: String,
    detail: String,
    state: State<'_, AppState>,
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
#[tauri::command]
pub async fn resolve_permission(id: String, allow: bool, state: State<'_, AppState>) -> Result<()> {
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
pub async fn consume(state: &AppState, project_id: &str, action: &str) -> Result<()> {
    if !settings(state).await?.required(action) {
        sqlx::query("INSERT INTO permissions(id,project_id,action,detail,decision,created_at) VALUES(?,?,?,?,?,?)").bind(uuid::Uuid::new_v4().to_string()).bind(project_id).bind(action).bind("Confirmation disabled in settings").bind("automatic").bind(chrono::Utc::now().to_rfc3339()).execute(&state.pool).await?;
        return Ok(());
    }
    let mut grants = state.grants.lock().await;
    consume_from(&mut grants, project_id, action)
}
fn consume_from(
    grants: &mut std::collections::HashMap<String, Grant>,
    project_id: &str,
    action: &str,
) -> Result<()> {
    let id = grants
        .iter()
        .find(|(_, g)| {
            g.project_id == project_id
                && g.action == action
                && g.approved
                && g.expires > Instant::now()
        })
        .map(|(id, _)| id.clone());
    if let Some(id) = id {
        grants.remove(&id);
        Ok(())
    } else {
        Err(AppError::Permission(action.to_owned()))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn confirmation_presets_and_overrides() {
        let mut s = ConfirmationSettings::default();
        assert!(ACTIONS.iter().all(|a| s.required(a)));
        s.mode = "cli".into();
        assert!(s.required("run_agent"));
        assert!(!s.required("modify_project"));
        s.mode = "none".into();
        assert!(ACTIONS.iter().all(|a| !s.required(a)));
        s.overrides.insert("modify_project".into(), true);
        assert!(s.required("modify_project"));
        assert!(!s.required("run_agent"));
    }
    #[test]
    fn approval_cannot_be_bypassed_or_replayed() {
        let mut map = std::collections::HashMap::new();
        map.insert(
            "a".into(),
            Grant {
                project_id: "p".into(),
                action: "modify_project".into(),
                approved: false,
                expires: Instant::now() + Duration::from_secs(30),
            },
        );
        assert!(consume_from(&mut map, "p", "modify_project").is_err());
        map.get_mut("a").unwrap().approved = true;
        assert!(consume_from(&mut map, "q", "modify_project").is_err());
        assert!(consume_from(&mut map, "p", "run_agent").is_err());
        assert!(consume_from(&mut map, "p", "modify_project").is_ok());
        assert!(consume_from(&mut map, "p", "modify_project").is_err());
    }
    #[test]
    fn expired_grant_rejected() {
        let mut map = std::collections::HashMap::new();
        map.insert(
            "a".into(),
            Grant {
                project_id: "p".into(),
                action: "run_agent".into(),
                approved: true,
                expires: Instant::now() - Duration::from_secs(1),
            },
        );
        assert!(consume_from(&mut map, "p", "run_agent").is_err());
    }
}
