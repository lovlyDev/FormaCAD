//! Host side of `formacad:plugin@0.1` imports. Every function re-checks its
//! permission even though the linker only exposes granted interfaces.
use super::super::{
    package::check_path,
    permissions::{Permission, PermissionSet},
};
use base64::Engine as _;
use std::{collections::BTreeMap, path::PathBuf};
use wasmtime::{ResourceLimiter, StoreLimits};

wasmtime::component::bindgen!({
    path: "../../../docs/plugin-api/wit",
    world: "formacad:plugin/host",
});

pub use formacad::plugin::types::Level;

pub const MAX_STORAGE_BYTES: usize = 10 * 1024 * 1024;
const MAX_NOTIFICATIONS: usize = 5;
const MAX_LOG_LINES: usize = 200;
const MAX_LOG_LINE: usize = 2000;
const MAX_RESPONSE_BYTES: usize = 10 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Notice {
    pub level: &'static str,
    pub message: String,
}

fn level_name(level: Level) -> &'static str {
    match level {
        Level::Info => "info",
        Level::Warning => "warning",
        Level::Error => "error",
    }
}

/// Memory limiter that remembers whether it refused growth, so the error can
/// say "memory limit" instead of a generic trap.
pub struct Limiter {
    pub inner: StoreLimits,
    pub memory_denied: bool,
}

impl ResourceLimiter for Limiter {
    fn memory_growing(
        &mut self,
        current: usize,
        desired: usize,
        maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        let allowed = self.inner.memory_growing(current, desired, maximum)?;
        self.memory_denied |= !allowed;
        Ok(allowed)
    }
    fn table_growing(
        &mut self,
        current: usize,
        desired: usize,
        maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        self.inner.table_growing(current, desired, maximum)
    }
}

/// Inputs and outputs of one plugin call.
pub struct HostState {
    pub limiter: Limiter,
    pub plugin_id: String,
    pub granted: PermissionSet,
    pub package_dir: PathBuf,
    pub data_dir: PathBuf,
    pub document: Option<String>,
    pub selection: String,
    pub notices: Vec<Notice>,
    pub logs: Vec<(Level, String)>,
    pub network: Option<tokio::runtime::Handle>,
    storage: Option<BTreeMap<String, String>>,
}

impl HostState {
    pub fn new(
        plugin_id: &str,
        granted: PermissionSet,
        package_dir: PathBuf,
        data_dir: PathBuf,
        memory_bytes: usize,
    ) -> Self {
        Self {
            limiter: Limiter {
                inner: wasmtime::StoreLimitsBuilder::new()
                    .memory_size(memory_bytes)
                    .instances(16)
                    .tables(16)
                    .memories(4)
                    .build(),
                memory_denied: false,
            },
            plugin_id: plugin_id.to_owned(),
            granted,
            package_dir,
            data_dir,
            document: None,
            selection: "[]".into(),
            notices: vec![],
            logs: vec![],
            network: None,
            storage: None,
        }
    }

    fn has(&self, permission: &Permission) -> bool {
        self.granted.contains(permission)
    }

    fn storage_path(&self) -> PathBuf {
        self.data_dir.join("storage.json")
    }

    fn storage(&mut self) -> &mut BTreeMap<String, String> {
        if self.storage.is_none() {
            let loaded = std::fs::read(self.storage_path())
                .ok()
                .and_then(|bytes| serde_json::from_slice(&bytes).ok())
                .unwrap_or_default();
            self.storage = Some(loaded);
        }
        self.storage.get_or_insert_with(Default::default)
    }

    fn save_storage(&mut self) -> Result<(), String> {
        let bytes = serde_json::to_vec(self.storage()).map_err(|e| e.to_string())?;
        std::fs::create_dir_all(&self.data_dir).map_err(|e| e.to_string())?;
        super::super::registry::write_atomic(&self.storage_path(), &bytes).map_err(|e| e.message)
    }
}

fn valid_storage_key(key: &str) -> bool {
    (1..=128).contains(&key.len())
        && key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

impl formacad::plugin::types::Host for HostState {}

impl formacad::plugin::log::Host for HostState {
    fn write(&mut self, level: Level, message: String) {
        if self.logs.len() < MAX_LOG_LINES {
            let mut message = message;
            if message.len() > MAX_LOG_LINE {
                let mut end = MAX_LOG_LINE;
                while !message.is_char_boundary(end) {
                    end -= 1;
                }
                message.truncate(end);
            }
            self.logs.push((level, message));
        }
    }
}

impl formacad::plugin::assets::Host for HostState {
    fn read(&mut self, path: String) -> Result<Vec<u8>, String> {
        check_path(&path).map_err(|error| error.message)?;
        if !(path.starts_with("assets/") || path.starts_with("locales/")) {
            return Err("Only assets/ and locales/ can be read".into());
        }
        std::fs::read(self.package_dir.join(&path)).map_err(|_| format!("{path} not found"))
    }
}

impl formacad::plugin::model::Host for HostState {
    fn document(&mut self) -> Result<String, String> {
        if !self.has(&Permission::ModelRead) {
            return Err("model.read is not granted".into());
        }
        self.document
            .clone()
            .ok_or_else(|| "The project has no CAD IR v2 model".into())
    }
}

impl formacad::plugin::selection::Host for HostState {
    fn current(&mut self) -> String {
        if self.has(&Permission::SelectionRead) {
            self.selection.clone()
        } else {
            "[]".into()
        }
    }
}

impl formacad::plugin::storage::Host for HostState {
    fn get(&mut self, key: String) -> Option<Vec<u8>> {
        if !self.has(&Permission::Storage) || !valid_storage_key(&key) {
            return None;
        }
        let encoded = self.storage().get(&key)?.clone();
        base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .ok()
    }

    fn set(&mut self, key: String, value: Vec<u8>) -> Result<(), String> {
        if !self.has(&Permission::Storage) {
            return Err("storage is not granted".into());
        }
        if !valid_storage_key(&key) {
            return Err("Invalid storage key".into());
        }
        let encoded = base64::engine::general_purpose::STANDARD.encode(&value);
        let used: usize = self
            .storage()
            .iter()
            .filter(|(existing, _)| **existing != key)
            .map(|(k, v)| k.len() + v.len() * 3 / 4)
            .sum();
        if used + key.len() + value.len() > MAX_STORAGE_BYTES {
            return Err("Plugin storage is limited to 10 MB".into());
        }
        self.storage().insert(key, encoded);
        self.save_storage()
    }

    fn remove(&mut self, key: String) {
        if self.has(&Permission::Storage) && self.storage().remove(&key).is_some() {
            let _ = self.save_storage();
        }
    }

    fn keys(&mut self) -> Vec<String> {
        if !self.has(&Permission::Storage) {
            return vec![];
        }
        self.storage().keys().cloned().collect()
    }
}

impl formacad::plugin::notifications::Host for HostState {
    fn notify(&mut self, level: Level, message: String) {
        if self.has(&Permission::Notifications) && self.notices.len() < MAX_NOTIFICATIONS {
            let message: String = message.chars().take(300).collect();
            self.notices.push(Notice {
                level: level_name(level),
                message,
            });
        }
    }
}

impl formacad::plugin::network::Host for HostState {
    fn fetch(
        &mut self,
        request: formacad::plugin::network::Request,
    ) -> Result<formacad::plugin::network::Response, String> {
        let url = url::Url::parse(&request.url).map_err(|_| "Invalid URL".to_string())?;
        let host = url.host_str().unwrap_or_default().to_owned();
        if url.scheme() != "https" || url.port().is_some() || !url.username().is_empty() {
            return Err("Only https:// URLs on the default port are allowed".into());
        }
        if !self.has(&Permission::Network(host.clone())) {
            return Err(format!("network:{host} is not granted"));
        }
        let method = reqwest::Method::from_bytes(request.method.to_ascii_uppercase().as_bytes())
            .ok()
            .filter(|m| ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD"].contains(&m.as_str()))
            .ok_or_else(|| "Unsupported HTTP method".to_string())?;
        let handle = self
            .network
            .clone()
            .ok_or_else(|| "Network is unavailable".to_string())?;
        handle.block_on(async move {
            if rustls::crypto::CryptoProvider::get_default().is_none() {
                let _ = rustls::crypto::ring::default_provider().install_default();
            }
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .map_err(|e| e.to_string())?;
            let mut builder = client.request(method, url);
            for (name, value) in request.headers {
                if !name.eq_ignore_ascii_case("host") && !name.eq_ignore_ascii_case("cookie") {
                    builder = builder.header(name, value);
                }
            }
            if let Some(body) = request.body {
                builder = builder.body(body);
            }
            let mut response = builder.send().await.map_err(|e| e.to_string())?;
            let status = response.status().as_u16();
            let headers = response
                .headers()
                .iter()
                .filter_map(|(k, v)| Some((k.to_string(), v.to_str().ok()?.to_owned())))
                .collect();
            let mut body = Vec::new();
            while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
                body.extend_from_slice(&chunk);
                if body.len() > MAX_RESPONSE_BYTES {
                    return Err("Response is larger than 10 MB".to_string());
                }
            }
            Ok(formacad::plugin::network::Response {
                status,
                headers,
                body,
            })
        })
    }
}

/// Add only the interfaces this plugin was granted.
pub fn link(
    linker: &mut wasmtime::component::Linker<HostState>,
    granted: &PermissionSet,
) -> wasmtime::Result<()> {
    use formacad::plugin::*;
    use wasmtime::component::HasSelf;
    types::add_to_linker::<_, HasSelf<_>>(linker, |s| s)?;
    log::add_to_linker::<_, HasSelf<_>>(linker, |s| s)?;
    assets::add_to_linker::<_, HasSelf<_>>(linker, |s| s)?;
    if granted.contains(&Permission::ModelRead) {
        model::add_to_linker::<_, HasSelf<_>>(linker, |s| s)?;
    }
    if granted.contains(&Permission::SelectionRead) {
        selection::add_to_linker::<_, HasSelf<_>>(linker, |s| s)?;
    }
    if granted.contains(&Permission::Storage) {
        storage::add_to_linker::<_, HasSelf<_>>(linker, |s| s)?;
    }
    if granted.contains(&Permission::Notifications) {
        notifications::add_to_linker::<_, HasSelf<_>>(linker, |s| s)?;
    }
    if granted.iter().any(|p| matches!(p, Permission::Network(_))) {
        network::add_to_linker::<_, HasSelf<_>>(linker, |s| s)?;
    }
    Ok(())
}
