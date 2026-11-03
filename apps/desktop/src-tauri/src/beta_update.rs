//! Private beta update channel for supporters.
//!
//! The stable channel is the updater endpoint baked into `tauri.conf.json`. The beta
//! channel asks the account server instead, with the signed-in bearer token: the server
//! re-checks supporter status and serves both the manifest and the installers. The token
//! stays in Rust; the WebView receives only the same update handle `check()` would return,
//! and the stored request headers authorise the later download too.

use serde::Serialize;
use std::time::Duration;
use tauri::{Manager, ResourceId, Runtime, Webview};
use tauri_plugin_updater::UpdaterExt;

use crate::account::{commands::access_token, AccountState};

/// Mirrors the updater plugin's check metadata so the JS `Update` class can wrap it.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BetaUpdate {
    rid: ResourceId,
    current_version: String,
    version: String,
    date: Option<String>,
    body: Option<String>,
    raw_json: serde_json::Value,
}

pub fn beta_endpoint(server: &str) -> String {
    format!(
        "{}/api/v1/desktop/beta/latest.json",
        server.trim_end_matches('/')
    )
}

#[tauri::command]
pub async fn beta_update_check<R: Runtime>(
    webview: Webview<R>,
    account: tauri::State<'_, AccountState>,
) -> Result<Option<BetaUpdate>, String> {
    let (server, token) = access_token(&account).await.map_err(|e| e.to_string())?;
    let endpoint = url::Url::parse(&beta_endpoint(&server)).map_err(|e| e.to_string())?;
    let updater = webview
        .updater_builder()
        .endpoints(vec![endpoint])
        .and_then(|b| b.header("Authorization", format!("Bearer {token}")))
        .map(|b| b.timeout(Duration::from_secs(15)))
        .and_then(|b| b.build())
        .map_err(|e| e.to_string())?;
    let Some(update) = updater.check().await.map_err(|e| e.to_string())? else {
        return Ok(None);
    };
    Ok(Some(BetaUpdate {
        current_version: update.current_version.clone(),
        version: update.version.clone(),
        date: None,
        body: update.body.clone(),
        raw_json: update.raw_json.clone(),
        rid: webview.resources_table().add(update),
    }))
}

#[cfg(test)]
mod tests {
    use super::beta_endpoint;

    #[test]
    fn beta_endpoint_is_on_the_account_server() {
        assert_eq!(
            beta_endpoint("https://formacad.ru"),
            "https://formacad.ru/api/v1/desktop/beta/latest.json"
        );
        assert_eq!(
            beta_endpoint("http://127.0.0.1:8080/"),
            "http://127.0.0.1:8080/api/v1/desktop/beta/latest.json"
        );
    }
}
