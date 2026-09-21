use crate::core::{AppError, AppState, Result};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path};

#[derive(Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Preferences(pub BTreeMap<String, String>);

impl Preferences {
    fn validate(&self) -> Result<()> {
        for (key, value) in &self.0 {
            let valid = match key.as_str() {
                "forma.locale" => matches!(value.as_str(), "ru" | "en"),
                "forma.theme" => matches!(value.as_str(), "light" | "dark"),
                _ => {
                    key.starts_with("forma.ui.")
                        && key.len() <= 256
                        && value.len() <= 32 * 1024 * 1024
                }
            };
            if !valid {
                return Err(AppError::Invalid("Invalid interface preference".into()));
            }
        }
        Ok(())
    }
}

pub async fn save_preferences(pool: &sqlx::SqlitePool, value: &Preferences) -> Result<()> {
    value.validate()?;
    sqlx::query("INSERT INTO settings(key,value) VALUES('ui_preferences',?) ON CONFLICT(key) DO UPDATE SET value=excluded.value")
        .bind(serde_json::to_string(value)?)
        .execute(pool).await?;
    Ok(())
}

#[tauri::command]
pub async fn get_ui_preferences(state: tauri::State<'_, AppState>) -> Result<Preferences> {
    let json: Option<String> =
        sqlx::query_scalar("SELECT value FROM settings WHERE key='ui_preferences'")
            .fetch_optional(&state.pool)
            .await?;
    Ok(json
        .map(|s| serde_json::from_str(&s))
        .transpose()?
        .unwrap_or_default())
}

#[tauri::command]
pub async fn set_ui_preferences(
    state: tauri::State<'_, AppState>,
    value: Preferences,
) -> Result<()> {
    save_preferences(&state.pool, &value).await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSupport {
    configured: bool,
    supported: bool,
}

#[tauri::command]
pub fn update_support(app: tauri::AppHandle) -> UpdateSupport {
    let config = app.config().plugins.0.get("updater");
    UpdateSupport {
        configured: config
            .and_then(|v| v.get("pubkey"))
            .and_then(|v| v.as_str())
            .is_some_and(|s| !s.is_empty())
            && config
                .and_then(|v| v.get("endpoints"))
                .and_then(|v| v.as_array())
                .is_some_and(|v| !v.is_empty()),
        supported: !cfg!(target_os = "linux") || std::env::var_os("APPIMAGE").is_some(),
    }
}

pub async fn backup_database(
    pool: &sqlx::SqlitePool,
    directory: &Path,
) -> Result<std::path::PathBuf> {
    std::fs::create_dir_all(directory)?;
    let destination = directory.join(format!(
        "before-update-{}-{}.sqlite",
        env!("CARGO_PKG_VERSION"),
        uuid::Uuid::new_v4()
    ));
    // VACUUM INTO includes committed WAL data, unlike copying the live .sqlite file.
    sqlx::query("VACUUM INTO ?")
        .bind(destination.to_string_lossy().as_ref())
        .execute(pool)
        .await?;
    Ok(destination)
}

#[tauri::command]
pub async fn prepare_update(
    state: tauri::State<'_, AppState>,
    preferences: Preferences,
) -> Result<()> {
    let _writes = state.writes.lock().await;
    if !state.tasks.lock().await.is_empty() {
        return Err(AppError::Invalid(
            "Finish the current operation before updating.".into(),
        ));
    }
    save_preferences(&state.pool, &preferences).await?;
    let data = state
        .root
        .parent()
        .ok_or_else(|| AppError::Invalid("Application data directory unavailable".into()))?;
    backup_database(&state.pool, &data.join("backups")).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn backup_preserves_settings_and_projects_across_reopen() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("forma.sqlite");
        let pool = crate::storage::open(&path).await.unwrap();
        let preferences = Preferences(BTreeMap::from([
            ("forma.locale".into(), "ru".into()),
            ("forma.theme".into(), "light".into()),
        ]));
        save_preferences(&pool, &preferences).await.unwrap();
        sqlx::query("INSERT INTO projects VALUES('id','Model','{\"revisions\":[1,2]}','now')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO settings VALUES('confirmation_settings','preserved')")
            .execute(&pool)
            .await
            .unwrap();
        let backup = backup_database(&pool, &temp.path().join("backups"))
            .await
            .unwrap();
        pool.close().await;
        for file in [path, backup] {
            let reopened = crate::storage::open(&file).await.unwrap();
            let stored: String =
                sqlx::query_scalar("SELECT value FROM settings WHERE key='ui_preferences'")
                    .fetch_one(&reopened)
                    .await
                    .unwrap();
            assert_eq!(stored, serde_json::to_string(&preferences).unwrap());
            let payload: String = sqlx::query_scalar("SELECT payload FROM projects WHERE id='id'")
                .fetch_one(&reopened)
                .await
                .unwrap();
            assert_eq!(payload, "{\"revisions\":[1,2]}");
            let policy: String =
                sqlx::query_scalar("SELECT value FROM settings WHERE key='confirmation_settings'")
                    .fetch_one(&reopened)
                    .await
                    .unwrap();
            assert_eq!(policy, "preserved");
            reopened.close().await;
        }
    }

    #[test]
    fn preferences_reject_unknown_keys_and_invalid_values() {
        assert!(Preferences(BTreeMap::from([("path".into(), "../".into())]))
            .validate()
            .is_err());
        assert!(
            Preferences(BTreeMap::from([("forma.locale".into(), "other".into())]))
                .validate()
                .is_err()
        );
    }
}
