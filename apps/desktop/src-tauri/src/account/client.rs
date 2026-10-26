use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::time::Duration;

use super::config::{CLIENT_ID, SCOPE};
use super::error::{AccountError, AccountResult};

/// HTTP client for the FormaCAD account server. Requests are bounded by short
/// timeouts so a slow or unreachable server never blocks the CAD interface.
pub struct ServerClient {
    http: reqwest::Client,
    pub origin: String,
}

#[derive(Deserialize)]
pub struct TokenSet {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub sub: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub email: Option<String>,
    pub locale: String,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EntitlementResponse {
    pub document: String,
    pub refresh_after: String,
    pub offline_until: String,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Jwks {
    pub keys: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
struct OAuthError {
    error: String,
}

impl ServerClient {
    pub fn new(origin: String) -> AccountResult<Self> {
        #[allow(clippy::collapsible_if)]
        if rustls::crypto::CryptoProvider::get_default().is_none() {
            let _ = rustls::crypto::ring::default_provider().install_default();
        }
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .connect_timeout(Duration::from_secs(8))
            .redirect(reqwest::redirect::Policy::none())
            .user_agent(concat!("FormaCAD/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| AccountError::Offline(e.to_string()))?;
        Ok(Self { http, origin })
    }

    pub fn authorize_url(&self, redirect_uri: &str, challenge: &str, state: &str, nonce: &str, device: &str) -> String {
        let mut url = url::Url::parse(&format!("{}/oauth/authorize", self.origin)).expect("origin is a valid URL");
        url.query_pairs_mut()
            .append_pair("response_type", "code")
            .append_pair("client_id", CLIENT_ID)
            .append_pair("redirect_uri", redirect_uri)
            .append_pair("code_challenge", challenge)
            .append_pair("code_challenge_method", "S256")
            .append_pair("state", state)
            .append_pair("nonce", nonce)
            .append_pair("scope", SCOPE)
            .append_pair("device_name", device);
        url.into()
    }

    async fn token_request(&self, form: &[(&str, &str)]) -> AccountResult<TokenSet> {
        let res = self
            .http
            .post(format!("{}/api/v1/oauth/token", self.origin))
            .form(form)
            .send()
            .await
            .map_err(|e| AccountError::Offline(e.to_string()))?;
        let status = res.status();
        if status.is_success() {
            return res.json().await.map_err(|e| AccountError::Invalid(e.to_string()));
        }
        let error = res.json::<OAuthError>().await.map(|e| e.error).unwrap_or_default();
        if error == "invalid_grant" {
            return Err(AccountError::SessionEnded);
        }
        Err(AccountError::Server(format!("token endpoint HTTP {status} {error}")))
    }

    pub async fn exchange_code(&self, code: &str, verifier: &str, redirect_uri: &str) -> AccountResult<TokenSet> {
        self.token_request(&[
            ("grant_type", "authorization_code"),
            ("client_id", CLIENT_ID),
            ("code", code),
            ("code_verifier", verifier),
            ("redirect_uri", redirect_uri),
        ])
        .await
    }

    pub async fn refresh(&self, refresh_token: &str) -> AccountResult<TokenSet> {
        self.token_request(&[("grant_type", "refresh_token"), ("client_id", CLIENT_ID), ("refresh_token", refresh_token)]).await
    }

    /// Tells the server the app stopped waiting, so the consent page closes too.
    pub async fn cancel_authorization(&self, state: &str) {
        let _ = self
            .http
            .post(format!("{}/api/v1/oauth/authorize/cancel", self.origin))
            .json(&serde_json::json!({ "client_id": CLIENT_ID, "state": state }))
            .send()
            .await;
    }

    /// Downloads the profile picture from our own server only and returns it as a
    /// `data:` URL: the WebView CSP allows no remote images, and stays that way.
    pub async fn avatar_data_url(&self, url: &str) -> Option<String> {
        let prefix = format!("{}/api/v1/avatars/", self.origin);
        if !url.starts_with(&prefix) {
            return None;
        }
        let res = self.http.get(url).send().await.ok()?;
        if !res.status().is_success() {
            return None;
        }
        let content_type = res.headers().get("content-type")?.to_str().ok()?.split(';').next()?.trim().to_string();
        if !matches!(content_type.as_str(), "image/png" | "image/jpeg" | "image/webp") {
            return None;
        }
        let bytes = res.bytes().await.ok()?;
        (bytes.len() <= 256 * 1024).then(|| format!("data:{content_type};base64,{}", STANDARD.encode(&bytes)))
    }

    pub async fn revoke(&self, refresh_token: &str) -> AccountResult<()> {
        self.http
            .post(format!("{}/api/v1/oauth/revoke", self.origin))
            .json(&serde_json::json!({ "token": refresh_token, "client_id": CLIENT_ID }))
            .send()
            .await
            .map_err(|e| AccountError::Offline(e.to_string()))?;
        Ok(())
    }

    async fn get<T: DeserializeOwned>(&self, path: &str, access: Option<&str>) -> AccountResult<T> {
        let mut req = self.http.get(format!("{}{path}", self.origin));
        if let Some(token) = access {
            req = req.bearer_auth(token);
        }
        let res = req.send().await.map_err(|e| AccountError::Offline(e.to_string()))?;
        match res.status().as_u16() {
            200 => res.json().await.map_err(|e| AccountError::Invalid(e.to_string())),
            401 => Err(AccountError::SessionEnded),
            s => Err(AccountError::Server(format!("{path} HTTP {s}"))),
        }
    }

    pub async fn profile(&self, access: &str) -> AccountResult<Profile> {
        self.get("/api/v1/desktop/me", Some(access)).await
    }

    pub async fn entitlements(&self, access: &str) -> AccountResult<EntitlementResponse> {
        self.get("/api/v1/desktop/entitlements", Some(access)).await
    }

    pub async fn entitlement_keys(&self) -> AccountResult<Jwks> {
        self.get("/api/v1/entitlements/keys", None).await
    }
}
