//! Optional FormaCAD account. Signing in never gates the local CAD core:
//! every command here can fail (offline, server down, signed out) while
//! projects, modeling and export keep working.
//!
//! Flow: OAuth 2.0 authorization code + PKCE (RFC 7636) for a native public
//! client (RFC 8252) via the system browser and a one-shot loopback listener
//! on 127.0.0.1. The refresh token lives in the OS credential store; the
//! access token only in memory; the signed entitlement document and the
//! public JWKS are cached locally for a bounded offline period.
mod cache;
mod client;
pub mod commands;
mod config;
#[cfg(test)]
mod e2e_tests;
mod entitlements;
mod error;
mod loopback;
mod pkce;
mod token_store;

pub use commands::AccountState;
