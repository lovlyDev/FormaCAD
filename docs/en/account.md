# FormaCAD account in the app

[Documentation](index.md) · [Русский](../ru/account.md) · [Settings and updates](settings-and-updates.md) · [Data](data.md)

## Purpose

Signing in to a FormaCAD account is optional. Projects, modeling, history, import and export work without an account, payment or connection to the server. The account is for support, subscriptions and future services. An ended subscription, an unreachable server or signing out never deletes or locks models.

## What the user does

1. Click the user icon in the top bar → “Sign in with browser”.
2. The system browser opens the FormaCAD site. Sign in with VK ID and click “Allow” on the “Sign in to FormaCAD on this computer” page. No password is typed into the app.
3. The browser hands a one-time code back to the app; the account dialog shows the name, plan, rights state and how long they stay valid offline.
4. “Check again” refreshes rights online. “Sign out” revokes the sign-in on the server and deletes local credentials. “Open account page” opens the site.

Waiting for approval can be cancelled; it ends by itself after 5 minutes.

## Technical route

- OAuth 2.0 Authorization Code + PKCE (S256) for a native public client without a secret (RFC 8252). Every attempt uses fresh `code_verifier`, `state` and `nonce`.
- The app opens `https://cad.kartochk1n.ru/oauth/authorize` in the system browser and listens once on `http://127.0.0.1:<random port>/callback`. A callback with a foreign `state` is ignored; `iss` must match the server (RFC 9207).
- Only a short-lived one-time code travels in the address bar. The code exchange and all tokens stay in the Rust host; the WebView receives only the profile and rights.
- The access token lives about 10 minutes, in memory only. The refresh token rotates on every refresh and is kept in the OS credential store (Windows Credential Manager, macOS Keychain, Linux kernel keyring). Reusing an old refresh token is treated as theft and revokes the whole sign-in.
- Rights arrive as a signed document (EdDSA/Ed25519, `typ: forma-entitlement+jwt`). The app checks the signature against the published keys at `/api/v1/entitlements/keys`, plus issuer, audience, version and expiry.

Code: [account](../../apps/desktop/src-tauri/src/account/), [interface](../../apps/desktop/src/features/account/).

## Data storage

| What | Where |
| --- | --- |
| Refresh token | OS credential store, service `studio.forma.cad.account` |
| Access token | Process memory only |
| Profile, signed rights document, public keys, last check time | Local SQLite `settings` table (key `account_cache`) |

Projects and models are never sent to the server.

## Offline and revocation

The rights document is valid for a bounded period (7 days by default) and asks for an online check after 24 hours. Offline, the app uses the saved document until it expires. Revocation (refund, disconnecting the device in the web account, signing out) takes effect at the next online check or at expiry — there is no unlimited offline mode. If the computer clock is set back before an already observed time, rights are treated as unconfirmed. This is not full protection against the owner of a fully controlled computer.

The app has no local paid features yet: rights are shown, but the free core never depends on them.

## Limitations

- The default server is `https://cad.kartochk1n.ru`. For development set `FORMA_ACCOUNT_SERVER=http://127.0.0.1:5173` (http is allowed only for 127.0.0.1).
- Real VK ID sign-in needs a configured APP_ID and a deployed server; locally it was verified with a VK ID stand-in and a test password account.
- On Linux the kernel keyring does not survive a reboot: sign in again afterwards.

## Verification

- Rust: `cargo test --lib account` — PKCE (RFC 7636 vector), loopback, signature checks (tampering, foreign key, rotation, expiry, clock rollback), unreachable server.
- End-to-end against a running server: `FORMA_ACCOUNT_E2E_SERVER=… FORMA_ACCOUNT_E2E_EMAIL=… FORMA_ACCOUNT_E2E_PASSWORD=… cargo test --lib account -- --include-ignored`.
- Interface: `npm test -- src/features/account`.
