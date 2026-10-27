//! Trusted public keys of the plugin catalog (Ed25519, separate from account
//! token keys).
//!
//! The private half lives only on the catalog server
//! (`PLUGIN_INDEX_SIGNING_KEYS`); `GET https://plugins.formacad.ru/api/v1/catalog/keys`
//! publishes the same bytes. Rotation: add the new key next to the old one,
//! ship a release, then switch the server's `keyId`.
pub const CATALOG_KEYS: &[(&str, [u8; 32])] = &[(
    "catalog-20261026",
    [
        0xb9, 0x0a, 0xfc, 0xad, 0xd1, 0x45, 0xbc, 0xb7, 0xa8, 0x55, 0x65, 0x07, 0x7a, 0x7a, 0xe2,
        0x4b, 0x73, 0x50, 0x1a, 0x3f, 0x09, 0x06, 0xda, 0x66, 0xea, 0xeb, 0x31, 0xfc, 0x39, 0x58,
        0x65, 0x3e,
    ],
)];

pub const CATALOG_HOST: &str = "plugins.formacad.ru";

#[cfg(test)]
mod tests {
    use base64::Engine as _;

    /// Same bytes as the server publishes in `/api/v1/catalog/keys` (`publicKeyBase64`).
    #[test]
    fn production_key_matches_published_value() {
        let published = base64::engine::general_purpose::STANDARD
            .decode("uQr8rdFFvLeoVWUHenriS3NQGj8JBtpm6usx/DlYZT4=")
            .unwrap();
        assert_eq!(super::CATALOG_KEYS[0].1.as_slice(), published.as_slice());
        assert!(ed25519_dalek::VerifyingKey::from_bytes(&super::CATALOG_KEYS[0].1).is_ok());
    }
}
