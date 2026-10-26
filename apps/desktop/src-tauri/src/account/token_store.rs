use super::error::{AccountError, AccountResult};

/// The refresh token is kept only in the OS credential store (Windows
/// Credential Manager, macOS Keychain, Linux kernel keyring) — never in
/// localStorage, SQLite or a plain JSON file.
const SERVICE: &str = "studio.forma.cad.account";

fn entry(server: &str) -> AccountResult<keyring::Entry> {
    keyring::Entry::new(SERVICE, server).map_err(|e| AccountError::Storage(e.to_string()))
}

pub fn load(server: &str) -> AccountResult<Option<String>> {
    match entry(server)?.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(AccountError::Storage(e.to_string())),
    }
}

pub fn save(server: &str, refresh_token: &str) -> AccountResult<()> {
    entry(server)?.set_password(refresh_token).map_err(|e| AccountError::Storage(e.to_string()))
}

pub fn clear(server: &str) -> AccountResult<()> {
    match entry(server)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(AccountError::Storage(e.to_string())),
    }
}
