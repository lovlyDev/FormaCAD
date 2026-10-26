//! Trusted public keys of the plugin catalog (Ed25519, separate from account
//! token keys).
//!
//! TODO(owner, lovlyDev): put the real catalog public key here before the
//! catalog opens. Generate the pair on the catalog server, keep the private
//! key there only, and add `(key id, 32-byte public key)` below. Until then
//! catalog installs fail with `PLUGIN_CATALOG_KEY_MISSING`; installing a
//! `.cadplugin` file still works. Rotation: add the new key next to the old
//! one, ship a release, then switch the server's `keyId`.
pub const CATALOG_KEYS: &[(&str, [u8; 32])] = &[];

pub const CATALOG_HOST: &str = "plugins.formacad.ru";
