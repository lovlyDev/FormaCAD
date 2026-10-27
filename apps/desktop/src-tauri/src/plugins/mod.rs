//! Plugin platform (plugin-api 0.1): `.cadplugin` packages, WASM components
//! in a Wasmtime sandbox, permissions, catalog links. Contract:
//! `docs/ru/plugin-platform.md`, WIT: `docs/plugin-api/wit/formacad-plugin.wit`.
pub mod apply;
pub mod catalog;
pub mod error;
pub mod forms;
pub mod ipc;
pub mod journal;
pub mod manifest;
pub mod package;
pub mod panel_view;
pub mod permissions;
pub mod registry;
pub mod runtime;
pub mod service;
#[cfg(test)]
mod tests;
pub mod wasm_check;

use std::sync::{Arc, Mutex};

/// Tauri-managed plugin state.
pub struct PluginState {
    pub service: Arc<service::PluginService>,
    /// `formacad://` link received at start-up, waiting for the UI.
    pub pending_link: Mutex<Option<String>>,
}

impl PluginState {
    pub fn new(profile: &std::path::Path) -> Self {
        Self {
            service: Arc::new(service::PluginService::new(profile.join("plugins"))),
            pending_link: Mutex::new(None),
        }
    }

    /// Keep only well-formed install links; anything else is ignored.
    pub fn offer_link(&self, url: &str) -> bool {
        if catalog::parse_link(url).is_ok() {
            *self.pending_link.lock().unwrap_or_else(|p| p.into_inner()) = Some(url.to_owned());
            true
        } else {
            false
        }
    }
}
