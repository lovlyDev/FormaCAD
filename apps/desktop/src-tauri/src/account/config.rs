use super::error::{AccountError, AccountResult};

/// Production account server (single origin for site, API and OAuth).
pub const DEFAULT_SERVER: &str = "https://formacad.ru";
pub const CLIENT_ID: &str = "forma-desktop";
pub const SCOPE: &str = "openid profile entitlements offline_access";

/// `FORMA_ACCOUNT_SERVER` overrides the server for development and staging.
/// Only https origins, or plain http on the loopback IP, are accepted.
pub fn server_origin() -> AccountResult<String> {
    let raw = std::env::var("FORMA_ACCOUNT_SERVER").unwrap_or_else(|_| DEFAULT_SERVER.to_string());
    normalize_origin(&raw)
}

pub fn normalize_origin(raw: &str) -> AccountResult<String> {
    let url = url::Url::parse(raw.trim())
        .map_err(|_| AccountError::Invalid("Invalid account server URL".into()))?;
    let loopback = matches!(url.host_str(), Some("127.0.0.1") | Some("[::1]"));
    let allowed = url.scheme() == "https" || (url.scheme() == "http" && loopback);
    if !allowed || url.username() != "" || url.password().is_some() || url.host_str().is_none() {
        return Err(AccountError::Invalid(
            "Account server must use https".into(),
        ));
    }
    Ok(url.origin().ascii_serialization())
}

/// Human readable device label shown in the web account's device list.
pub fn device_name() -> String {
    let host = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_default();
    let host: String = host.chars().filter(|c| !c.is_control()).take(40).collect();
    if host.is_empty() {
        "FormaCAD Desktop".into()
    } else {
        format!("FormaCAD · {host}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_https_or_loopback_http() {
        assert_eq!(
            normalize_origin("https://formacad.ru/").unwrap(),
            "https://formacad.ru"
        );
        assert_eq!(
            normalize_origin("http://127.0.0.1:5173").unwrap(),
            "http://127.0.0.1:5173"
        );
        assert!(normalize_origin("http://formacad.ru").is_err());
        assert!(normalize_origin("http://localhost:5173").is_err());
        assert!(normalize_origin("https://user:pw@kartochk1n.ru").is_err());
        assert!(normalize_origin("file:///etc/passwd").is_err());
    }
}
