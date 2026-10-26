//! End-to-end check of the desktop account client against a running FormaCAD
//! server (not run by default). The browser step is performed with plain HTTP
//! using a password test account on the server:
//!
//!   FORMA_ACCOUNT_E2E_SERVER=http://127.0.0.1:5173 \
//!   FORMA_ACCOUNT_E2E_EMAIL=... FORMA_ACCOUNT_E2E_PASSWORD=... \
//!   cargo test --lib account::e2e_tests -- --ignored --nocapture
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::watch;

use super::client::ServerClient;
use super::entitlements::{evaluate, DocState};
use super::error::AccountError;
use super::loopback::Loopback;
use super::pkce;

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.is_empty())
}

#[tokio::test]
#[ignore = "needs a running FormaCAD server and a test account"]
async fn sign_in_refresh_entitlements_and_revoke_against_server() {
    let (Some(server), Some(email), Some(password)) = (
        env("FORMA_ACCOUNT_E2E_SERVER"),
        env("FORMA_ACCOUNT_E2E_EMAIL"),
        env("FORMA_ACCOUNT_E2E_PASSWORD"),
    ) else {
        panic!(
            "set FORMA_ACCOUNT_E2E_SERVER, FORMA_ACCOUNT_E2E_EMAIL and FORMA_ACCOUNT_E2E_PASSWORD"
        );
    };
    let server = super::config::normalize_origin(&server).unwrap();
    let client = ServerClient::new(server.clone()).unwrap();

    // --- the "system browser": a cookie session on the website ---
    let browser = reqwest::Client::builder()
        .cookie_store(true)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let login = browser
        .post(format!("{server}/api/v1/auth/login"))
        .header("origin", &server)
        .json(&serde_json::json!({ "email": email, "password": password }))
        .send()
        .await
        .unwrap();
    assert!(
        login.status().is_success(),
        "web login failed: {}",
        login.status()
    );
    let session: serde_json::Value = browser
        .get(format!("{server}/api/v1/session"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let csrf = session["csrfToken"].as_str().unwrap().to_string();

    // --- desktop: PKCE + loopback, then "open" the authorize URL ---
    let pair = pkce::generate();
    let loopback = Loopback::bind().await.unwrap();
    let redirect = loopback.redirect_uri.clone();
    let (_cancel_tx, cancel_rx) = watch::channel(false);
    let state = pair.state.clone();
    let waiter = tokio::spawn(async move {
        loopback
            .wait(&state, Duration::from_secs(30), cancel_rx)
            .await
    });
    let authorize = client.authorize_url(
        &redirect,
        &pair.challenge,
        &pair.state,
        &pair.nonce,
        "FormaCAD · e2e",
    );
    let to_consent = browser.get(&authorize).send().await.unwrap();
    let location = to_consent.headers()["location"]
        .to_str()
        .unwrap()
        .to_string();
    let request_id = location
        .split("request=")
        .nth(1)
        .expect("consent redirect")
        .to_string();
    let decision: serde_json::Value = browser
        .post(format!(
            "{server}/api/v1/desktop-auth/requests/{request_id}/decision"
        ))
        .header("origin", &server)
        .header("x-csrf-token", &csrf)
        .json(&serde_json::json!({ "approve": true }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let back = decision["redirectUrl"].as_str().unwrap();
    assert!(back.starts_with(&redirect), "redirect goes to our loopback");
    assert!(
        !back.contains("access_token") && !back.contains("refresh_token"),
        "only a one-time code in the URL"
    );
    reqwest::get(back).await.unwrap(); // the browser following the redirect
    let callback = waiter.await.unwrap().unwrap();
    assert_eq!(callback.iss.as_deref(), Some(server.as_str()));

    // --- token exchange; the code is single use ---
    let tokens = client
        .exchange_code(&callback.code, &pair.verifier, &redirect)
        .await
        .unwrap();
    assert!(matches!(
        client
            .exchange_code(&callback.code, &pair.verifier, &redirect)
            .await,
        Err(AccountError::SessionEnded)
    ));
    // Replaying the code revoked that family: sign in again for the rest of the test.
    assert!(matches!(
        client.profile(&tokens.access_token).await,
        Err(AccountError::SessionEnded)
    ));

    let pair = pkce::generate();
    let loopback = Loopback::bind().await.unwrap();
    let redirect = loopback.redirect_uri.clone();
    let (_tx, rx) = watch::channel(false);
    let state = pair.state.clone();
    let waiter =
        tokio::spawn(async move { loopback.wait(&state, Duration::from_secs(30), rx).await });
    let location = browser
        .get(client.authorize_url(
            &redirect,
            &pair.challenge,
            &pair.state,
            &pair.nonce,
            "FormaCAD · e2e",
        ))
        .send()
        .await
        .unwrap()
        .headers()["location"]
        .to_str()
        .unwrap()
        .to_string();
    let request_id = location.split("request=").nth(1).unwrap().to_string();
    let decision: serde_json::Value = browser
        .post(format!(
            "{server}/api/v1/desktop-auth/requests/{request_id}/decision"
        ))
        .header("origin", &server)
        .header("x-csrf-token", &csrf)
        .json(&serde_json::json!({ "approve": true }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    reqwest::get(decision["redirectUrl"].as_str().unwrap())
        .await
        .unwrap();
    let code = waiter.await.unwrap().unwrap().code;
    // A wrong verifier is rejected.
    assert!(matches!(
        client
            .exchange_code(&code, &pkce::generate().verifier, &redirect)
            .await,
        Err(AccountError::SessionEnded)
    ));

    // Fresh, correct exchange.
    let pair = pkce::generate();
    let loopback = Loopback::bind().await.unwrap();
    let redirect = loopback.redirect_uri.clone();
    let (_tx, rx) = watch::channel(false);
    let state = pair.state.clone();
    let waiter =
        tokio::spawn(async move { loopback.wait(&state, Duration::from_secs(30), rx).await });
    let location = browser
        .get(client.authorize_url(
            &redirect,
            &pair.challenge,
            &pair.state,
            &pair.nonce,
            "FormaCAD · e2e",
        ))
        .send()
        .await
        .unwrap()
        .headers()["location"]
        .to_str()
        .unwrap()
        .to_string();
    let request_id = location.split("request=").nth(1).unwrap().to_string();
    let decision: serde_json::Value = browser
        .post(format!(
            "{server}/api/v1/desktop-auth/requests/{request_id}/decision"
        ))
        .header("origin", &server)
        .header("x-csrf-token", &csrf)
        .json(&serde_json::json!({ "approve": true }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    reqwest::get(decision["redirectUrl"].as_str().unwrap())
        .await
        .unwrap();
    let code = waiter.await.unwrap().unwrap().code;
    let tokens = client
        .exchange_code(&code, &pair.verifier, &redirect)
        .await
        .unwrap();

    // --- profile + signed entitlements verified with the published keys ---
    let profile = client.profile(&tokens.access_token).await.unwrap();
    assert_eq!(profile.sub, session["user"]["id"].as_str().unwrap());
    let ent = client.entitlements(&tokens.access_token).await.unwrap();
    let jwks = client.entitlement_keys().await.unwrap();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let (doc_state, claims) = evaluate(&ent.document, &jwks.keys, &server, now, now);
    assert_eq!(doc_state, DocState::Valid);
    let claims = claims.unwrap();
    assert!(claims.capabilities.iter().any(|c| c.key == "account.basic"));
    assert_eq!(
        evaluate(&ent.document, &jwks.keys, &server, claims.exp + 1, now).0,
        DocState::Expired
    );

    // --- refresh rotation and reuse detection ---
    let rotated = client.refresh(&tokens.refresh_token).await.unwrap();
    assert_ne!(rotated.refresh_token, tokens.refresh_token);
    assert!(matches!(
        client.refresh(&tokens.refresh_token).await,
        Err(AccountError::SessionEnded)
    ));
    assert!(
        matches!(
            client.refresh(&rotated.refresh_token).await,
            Err(AccountError::SessionEnded)
        ),
        "reuse revokes the family"
    );

    println!(
        "e2e OK: user {} plan {} capabilities {:?}",
        profile.sub,
        claims.plan,
        claims
            .capabilities
            .iter()
            .map(|c| &c.key)
            .collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn unreachable_server_is_reported_as_offline() {
    let client = ServerClient::new("http://127.0.0.1:9".into()).unwrap();
    assert!(matches!(
        client.entitlement_keys().await,
        Err(AccountError::Offline(_))
    ));
}
