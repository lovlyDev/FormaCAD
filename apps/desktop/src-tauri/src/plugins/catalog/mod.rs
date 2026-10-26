//! Catalog installs: `formacad://plugins/install?id=&version=` links, the
//! signed index entry and the package download. A link never installs
//! anything by itself; it only produces a preview for the confirmation window.
pub mod keys;

use super::{
    error::{PluginError, Result},
    package::MAX_PACKAGE_BYTES,
    service::InstallPreview,
};
use base64::Engine as _;
use serde::Deserialize;

const MAX_ENTRY_BYTES: usize = 64 * 1024;
pub const ENTRY_FORMAT: &str = "formacad-plugin-index-entry/1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallLink {
    pub id: String,
    pub version: String,
}

fn link_error(message: &str) -> PluginError {
    PluginError::host("PLUGIN_LINK_INVALID", message)
}

fn valid_id(id: &str) -> bool {
    (3..=64).contains(&id.len())
        && !id.starts_with('-')
        && !id.ends_with('-')
        && !id.contains("--")
        && id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// Strict parser: scheme, host, path and exactly the two parameters.
pub fn parse_link(raw: &str) -> Result<InstallLink> {
    let url = url::Url::parse(raw).map_err(|_| link_error("Not a URL"))?;
    if url.scheme() != "formacad" || url.host_str() != Some("plugins") || url.path() != "/install" {
        return Err(link_error("Unsupported formacad:// link"));
    }
    let (mut id, mut version) = (None, None);
    for (key, value) in url.query_pairs() {
        let slot = match key.as_ref() {
            "id" => &mut id,
            "version" => &mut version,
            _ => return Err(link_error("Unexpected link parameter")),
        };
        if slot.replace(value.into_owned()).is_some() {
            return Err(link_error("Repeated link parameter"));
        }
    }
    let id = id.filter(|id| valid_id(id)).ok_or_else(|| link_error("Invalid plugin id"))?;
    let version = version
        .filter(|v| semver::Version::parse(v).is_ok_and(|parsed| parsed.pre.is_empty() && parsed.build.is_empty()))
        .ok_or_else(|| link_error("Invalid plugin version"))?;
    Ok(InstallLink { id, version })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SignedEntry {
    payload: String,
    signature: String,
    key_id: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexEntry {
    pub format: String,
    pub id: String,
    pub version: String,
    pub name: String,
    pub permissions: Vec<String>,
    pub sha256: String,
    pub size: u64,
    pub download_url: String,
    pub page_url: String,
    #[serde(default)]
    pub yanked: bool,
}

fn catalog_error(code: &str, message: impl Into<String>) -> PluginError {
    PluginError::host(code, message)
}

fn on_catalog(raw: &str) -> bool {
    url::Url::parse(raw).is_ok_and(|url| {
        url.scheme() == "https" && url.host_str() == Some(keys::CATALOG_HOST) && url.port().is_none() && url.username().is_empty()
    })
}

/// Verify a signed index entry against trusted keys and the requested link.
pub fn verify_entry(body: &[u8], link: &InstallLink, trusted: &[(&str, [u8; 32])]) -> Result<IndexEntry> {
    if trusted.is_empty() {
        return Err(catalog_error("PLUGIN_CATALOG_KEY_MISSING", "This build has no catalog key; install from a file instead"));
    }
    let signed: SignedEntry = serde_json::from_slice(body).map_err(|e| catalog_error("PLUGIN_CATALOG_INVALID", e.to_string()))?;
    let (_, key) = trusted
        .iter()
        .find(|(id, _)| *id == signed.key_id)
        .ok_or_else(|| catalog_error("PLUGIN_SIGNATURE_INVALID", "Unknown catalog key"))?;
    let b64 = base64::engine::general_purpose::STANDARD;
    let payload = b64.decode(&signed.payload).map_err(|_| catalog_error("PLUGIN_CATALOG_INVALID", "payload is not base64"))?;
    let signature = b64
        .decode(&signed.signature)
        .ok()
        .and_then(|bytes| ed25519_dalek::Signature::from_slice(&bytes).ok())
        .ok_or_else(|| catalog_error("PLUGIN_SIGNATURE_INVALID", "Malformed signature"))?;
    let key = ed25519_dalek::VerifyingKey::from_bytes(key).map_err(|_| catalog_error("PLUGIN_SIGNATURE_INVALID", "Bad catalog key"))?;
    key.verify_strict(&payload, &signature)
        .map_err(|_| catalog_error("PLUGIN_SIGNATURE_INVALID", "Index entry signature does not match"))?;
    let entry: IndexEntry = serde_json::from_slice(&payload).map_err(|e| catalog_error("PLUGIN_CATALOG_INVALID", e.to_string()))?;
    if entry.format != ENTRY_FORMAT || entry.id != link.id || entry.version != link.version {
        return Err(catalog_error("PLUGIN_CATALOG_INVALID", "Index entry does not match the link"));
    }
    if entry.yanked {
        return Err(catalog_error("PLUGIN_YANKED", "This version was withdrawn by the catalog"));
    }
    if !on_catalog(&entry.download_url) || !on_catalog(&entry.page_url) {
        return Err(catalog_error("PLUGIN_CATALOG_INVALID", "Package must come from the catalog server"));
    }
    if entry.size == 0 || entry.size > MAX_PACKAGE_BYTES as u64 || entry.sha256.len() != 64 || !entry.sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(catalog_error("PLUGIN_CATALOG_INVALID", "Bad package size or hash"));
    }
    Ok(entry)
}

/// The package must be exactly what the signed entry describes.
pub fn check_package(entry: &IndexEntry, bytes: &[u8]) -> Result<()> {
    use sha2::Digest;
    if bytes.len() as u64 != entry.size || format!("{:x}", sha2::Sha256::digest(bytes)) != entry.sha256.to_ascii_lowercase() {
        return Err(catalog_error("PLUGIN_HASH_MISMATCH", "Downloaded package does not match the signed hash"));
    }
    Ok(())
}

/// The manifest inside the package must agree with the signed entry.
pub fn check_preview(entry: &IndexEntry, preview: &InstallPreview) -> Result<()> {
    let mut signed = entry.permissions.clone();
    signed.sort();
    let mut declared = preview.permissions.clone();
    declared.sort();
    if preview.manifest.id != entry.id || preview.manifest.version != entry.version || signed != declared {
        return Err(catalog_error("PLUGIN_CATALOG_INVALID", "Package manifest differs from the signed index entry"));
    }
    Ok(())
}

async fn get_limited(client: &reqwest::Client, url: &str, limit: usize) -> Result<Vec<u8>> {
    let mut response = client
        .get(url)
        .send()
        .await
        .map_err(|e| catalog_error("PLUGIN_CATALOG_UNREACHABLE", e.to_string()))?;
    if !response.status().is_success() {
        let code = if response.status() == reqwest::StatusCode::NOT_FOUND { "PLUGIN_CATALOG_NOT_FOUND" } else { "PLUGIN_CATALOG_UNREACHABLE" };
        return Err(catalog_error(code, format!("HTTP {}", response.status())));
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|e| catalog_error("PLUGIN_CATALOG_UNREACHABLE", e.to_string()))? {
        body.extend_from_slice(&chunk);
        if body.len() > limit {
            return Err(catalog_error("PLUGIN_CATALOG_INVALID", "Response is too large"));
        }
    }
    Ok(body)
}

/// Fetch, verify and download. Returns the package bytes and the entry.
pub async fn download(link: &InstallLink) -> Result<(Vec<u8>, IndexEntry)> {
    if keys::CATALOG_KEYS.is_empty() {
        return Err(catalog_error("PLUGIN_CATALOG_KEY_MISSING", "This build has no catalog key; install from a file instead"));
    }
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .connect_timeout(std::time::Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| catalog_error("PLUGIN_CATALOG_UNREACHABLE", e.to_string()))?;
    let url = format!("https://{}/api/v1/plugins/{}/versions/{}", keys::CATALOG_HOST, link.id, link.version);
    let body = get_limited(&client, &url, MAX_ENTRY_BYTES).await?;
    let entry = verify_entry(&body, link, keys::CATALOG_KEYS)?;
    let package = get_limited(&client, &entry.download_url, MAX_PACKAGE_BYTES).await?;
    check_package(&entry, &package)?;
    Ok((package, entry))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn link() -> InstallLink {
        InstallLink { id: "simple-gear".into(), version: "0.1.0".into() }
    }

    fn signed(key: &SigningKey, payload: serde_json::Value) -> Vec<u8> {
        let bytes = payload.to_string().into_bytes();
        let b64 = base64::engine::general_purpose::STANDARD;
        serde_json::json!({"payload": b64.encode(&bytes), "signature": b64.encode(key.sign(&bytes).to_bytes()), "keyId": "test"})
            .to_string()
            .into_bytes()
    }

    fn payload() -> serde_json::Value {
        serde_json::json!({
            "format": ENTRY_FORMAT, "id": "simple-gear", "version": "0.1.0", "name": "Simple Gear",
            "author": {"name": "A", "github": "a"}, "api": "0.1", "minAppVersion": "1.3.3",
            "permissions": ["model.write"], "sha256": "ab".repeat(32), "size": 10,
            "downloadUrl": "https://plugins.formacad.ru/api/v1/plugins/simple-gear/versions/0.1.0/package",
            "pageUrl": "https://plugins.formacad.ru/plugins/simple-gear", "yanked": false,
            "publishedAt": "2026-10-26T00:00:00Z", "futureField": 1
        })
    }

    #[test]
    fn parses_only_strict_install_links() {
        assert_eq!(parse_link("formacad://plugins/install?id=simple-gear&version=0.1.0").unwrap(), link());
        for bad in [
            "https://plugins/install?id=simple-gear&version=0.1.0",
            "formacad://plugins/remove?id=simple-gear&version=0.1.0",
            "formacad://evil/install?id=simple-gear&version=0.1.0",
            "formacad://plugins/install?id=Simple&version=0.1.0",
            "formacad://plugins/install?id=../x&version=0.1.0",
            "formacad://plugins/install?id=simple-gear&version=latest",
            "formacad://plugins/install?id=simple-gear&version=0.1.0-beta",
            "formacad://plugins/install?id=simple-gear&version=0.1.0&run=1",
            "formacad://plugins/install?id=simple-gear&id=other&version=0.1.0",
        ] {
            assert!(parse_link(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn verifies_signature_and_entry_fields() {
        let key = SigningKey::from_bytes(&[7; 32]);
        let trusted = [("test", key.verifying_key().to_bytes())];
        let entry = verify_entry(&signed(&key, payload()), &link(), &trusted).unwrap();
        assert_eq!(entry.size, 10);

        let code = |body: Vec<u8>| verify_entry(&body, &link(), &trusted).unwrap_err().code;
        assert_eq!(verify_entry(&signed(&key, payload()), &link(), &[]).unwrap_err().code, "PLUGIN_CATALOG_KEY_MISSING");
        assert_eq!(code(signed(&SigningKey::from_bytes(&[8; 32]), payload())), "PLUGIN_SIGNATURE_INVALID");
        let mut tampered: serde_json::Value = serde_json::from_slice(&signed(&key, payload())).unwrap();
        let mut other = payload();
        other["permissions"] = serde_json::json!(["model.write", "network:evil.example.com"]);
        tampered["payload"] = base64::engine::general_purpose::STANDARD.encode(other.to_string()).into();
        assert_eq!(code(tampered.to_string().into_bytes()), "PLUGIN_SIGNATURE_INVALID");
        for (field, value, expected) in [
            ("version", serde_json::json!("0.2.0"), "PLUGIN_CATALOG_INVALID"),
            ("yanked", serde_json::json!(true), "PLUGIN_YANKED"),
            ("downloadUrl", serde_json::json!("https://github.com/x/y.cadplugin"), "PLUGIN_CATALOG_INVALID"),
            ("downloadUrl", serde_json::json!("http://plugins.formacad.ru/p"), "PLUGIN_CATALOG_INVALID"),
            ("size", serde_json::json!(MAX_PACKAGE_BYTES + 1), "PLUGIN_CATALOG_INVALID"),
        ] {
            let mut changed = payload();
            changed[field] = value;
            assert_eq!(code(signed(&key, changed)), expected, "{field}");
        }
    }

    #[test]
    fn package_hash_and_size_must_match() {
        use sha2::Digest;
        let bytes = b"0123456789";
        let mut value = payload();
        value["sha256"] = format!("{:x}", sha2::Sha256::digest(bytes)).into();
        let entry: IndexEntry = serde_json::from_value(value).unwrap();
        assert!(check_package(&entry, bytes).is_ok());
        assert_eq!(check_package(&entry, b"0123456780").unwrap_err().code, "PLUGIN_HASH_MISMATCH");
        assert_eq!(check_package(&entry, b"01234").unwrap_err().code, "PLUGIN_HASH_MISMATCH");
    }
}
