use serde::Serialize;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri_plugin_opener::OpenerExt;
use tokio::sync::{watch, Mutex};

use super::cache::{self, AccountCache};
use super::client::{Profile, ServerClient};
use super::config;
use super::entitlements::{evaluate, usable_capabilities, Claims, DocState};
use super::error::{AccountError, AccountResult};
use super::loopback::Loopback;
use super::pkce;
use super::token_store;
use crate::core::AppState;

const SIGN_IN_TIMEOUT: Duration = Duration::from_secs(300);

/// In-memory account state. The access token never reaches the WebView or disk.
#[derive(Default)]
pub struct AccountState {
    /// Serialises token use: rotated refresh tokens must never be sent twice.
    session: Mutex<Option<(String, Instant)>>,
    cancel: std::sync::Mutex<Option<watch::Sender<bool>>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountStatus {
    pub server: String,
    pub signed_in: bool,
    pub profile: Option<Profile>,
    pub plan: Option<String>,
    pub subscription_status: Option<String>,
    pub document_state: Option<DocState>,
    pub capabilities: Vec<String>,
    pub offline_until: Option<i64>,
    pub refresh_after: Option<i64>,
    pub synced_at: Option<i64>,
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

async fn current_cache(state: &AppState, server: &str) -> AccountResult<AccountCache> {
    Ok(cache::load(&state.pool)
        .await?
        .filter(|c| c.server == server)
        .unwrap_or_else(|| AccountCache {
            server: server.into(),
            ..Default::default()
        }))
}

fn build_status(server: &str, signed_in: bool, cache: &AccountCache) -> AccountStatus {
    let (doc_state, claims): (Option<DocState>, Option<Claims>) = match &cache.document {
        Some(doc) => {
            let (s, c) = evaluate(doc, &cache.jwks, server, now(), cache.max_seen);
            (Some(s), c)
        }
        None => (None, None),
    };
    AccountStatus {
        server: server.into(),
        signed_in,
        profile: cache.profile.clone(),
        plan: claims.as_ref().map(|c| c.plan.clone()),
        subscription_status: claims.as_ref().map(|c| c.subscription_status.clone()),
        capabilities: doc_state
            .map(|s| usable_capabilities(s, claims.as_ref()))
            .unwrap_or_default(),
        offline_until: claims.as_ref().map(|c| c.exp),
        refresh_after: claims.as_ref().map(|c| c.rfa),
        document_state: doc_state,
        synced_at: cache.synced_at,
    }
}

/// Fetches profile, entitlement document and public keys with a valid access token.
async fn sync(
    state: &AppState,
    client: &ServerClient,
    access: &str,
) -> AccountResult<AccountCache> {
    let mut profile = client.profile(access).await?;
    profile.avatar_url = match profile.avatar_url.as_deref() {
        Some(url) => client.avatar_data_url(url).await,
        None => None,
    };
    let ent = client.entitlements(access).await?;
    let jwks = client.entitlement_keys().await?;
    let mut cache = current_cache(state, &client.origin).await?;
    cache.profile = Some(profile);
    cache.document = Some(ent.document);
    cache.jwks = jwks.keys;
    cache.synced_at = Some(now());
    cache.max_seen = cache.max_seen.max(now());
    cache::save(&state.pool, &cache).await?;
    Ok(cache)
}

/// Offline-safe status from cache only. Never touches the network.
#[tauri::command]
pub async fn account_status(state: tauri::State<'_, AppState>) -> AccountResult<AccountStatus> {
    let server = config::server_origin()?;
    let signed_in = token_store::load(&server)
        .map(|t| t.is_some())
        .unwrap_or(false);
    let mut cache = current_cache(&state, &server).await?;
    if now() > cache.max_seen && cache.document.is_some() {
        cache.max_seen = now();
        cache::save(&state.pool, &cache).await?;
    }
    Ok(build_status(&server, signed_in, &cache))
}

/// Opens the system browser on the account site and waits for the one-time
/// code on the loopback listener. The password is never entered in the app.
#[tauri::command]
pub async fn account_sign_in(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    account: tauri::State<'_, AccountState>,
) -> AccountResult<AccountStatus> {
    let server = config::server_origin()?;
    let client = ServerClient::new(server.clone())?;
    let (tx, rx) = watch::channel(false);
    {
        let mut slot = account.cancel.lock().map_err(|_| AccountError::Busy)?;
        if slot.is_some() {
            return Err(AccountError::Busy);
        }
        *slot = Some(tx);
    }
    let result = async {
        let pair = pkce::generate();
        let loopback = Loopback::bind().await?;
        let redirect = loopback.redirect_uri.clone();
        let url = client.authorize_url(
            &redirect,
            &pair.challenge,
            &pair.state,
            &pair.nonce,
            &config::device_name(),
        );
        app.opener()
            .open_url(url, None::<&str>)
            .map_err(|e| AccountError::Storage(format!("cannot open browser: {e}")))?;
        let callback = match loopback.wait(&pair.state, SIGN_IN_TIMEOUT, rx).await {
            Ok(callback) => callback,
            Err(error) => {
                if matches!(error, AccountError::Cancelled | AccountError::Timeout) {
                    client.cancel_authorization(&pair.state).await;
                }
                return Err(error);
            }
        };
        if let Some(iss) = &callback.iss {
            // RFC 9207: the response must come from the server we started with.
            if iss.trim_end_matches('/') != server {
                return Err(AccountError::Invalid("issuer mismatch".into()));
            }
        }
        let mut session = account.session.lock().await;
        let tokens = client
            .exchange_code(&callback.code, &pair.verifier, &redirect)
            .await?;
        token_store::save(&server, &tokens.refresh_token)?;
        *session = Some((
            tokens.access_token.clone(),
            Instant::now() + Duration::from_secs(tokens.expires_in.saturating_sub(30)),
        ));
        let cache = sync(&state, &client, &tokens.access_token).await?;
        Ok(build_status(&server, true, &cache))
    }
    .await;
    if let Ok(mut slot) = account.cancel.lock() {
        *slot = None;
    }
    result
}

#[tauri::command]
pub fn account_cancel_sign_in(account: tauri::State<'_, AccountState>) {
    if let Ok(slot) = account.cancel.lock() {
        if let Some(tx) = slot.as_ref() {
            let _ = tx.send(true);
        }
    }
}

/// Online re-check: rotates the refresh token, refreshes profile and rights.
/// Offline keeps the cached (bounded) rights; a revoked session signs out.
#[tauri::command]
pub async fn account_refresh(
    state: tauri::State<'_, AppState>,
    account: tauri::State<'_, AccountState>,
) -> AccountResult<AccountStatus> {
    let server = config::server_origin()?;
    let client = ServerClient::new(server.clone())?;
    let mut session = account.session.lock().await;
    let access = match session.as_ref() {
        Some((token, until)) if *until > Instant::now() => token.clone(),
        _ => {
            let refresh = token_store::load(&server)?.ok_or(AccountError::SignedOut)?;
            match client.refresh(&refresh).await {
                Ok(tokens) => {
                    token_store::save(&server, &tokens.refresh_token)?;
                    *session = Some((
                        tokens.access_token.clone(),
                        Instant::now() + Duration::from_secs(tokens.expires_in.saturating_sub(30)),
                    ));
                    tokens.access_token
                }
                Err(AccountError::SessionEnded) => {
                    *session = None;
                    token_store::clear(&server)?;
                    return Err(AccountError::SessionEnded);
                }
                Err(e) => return Err(e),
            }
        }
    };
    match sync(&state, &client, &access).await {
        Ok(cache) => Ok(build_status(&server, true, &cache)),
        Err(AccountError::SessionEnded) => {
            *session = None;
            Err(AccountError::SessionEnded)
        }
        Err(e) => Err(e),
    }
}

/// Revokes the token family on the server (best effort) and forgets local credentials.
#[tauri::command]
pub async fn account_sign_out(
    state: tauri::State<'_, AppState>,
    account: tauri::State<'_, AccountState>,
) -> AccountResult<AccountStatus> {
    let server = config::server_origin()?;
    let mut session = account.session.lock().await;
    *session = None;
    if let Some(refresh) = token_store::load(&server)? {
        if let Ok(client) = ServerClient::new(server.clone()) {
            let _ = client.revoke(&refresh).await;
        }
    }
    token_store::clear(&server)?;
    cache::clear(&state.pool).await?;
    Ok(build_status(
        &server,
        false,
        &AccountCache {
            server: server.clone(),
            ..Default::default()
        },
    ))
}
