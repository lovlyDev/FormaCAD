//! `.cadplugin` archive inspection. Every limit is checked from the ZIP
//! directory before anything is decompressed, and again while reading so a
//! lying header cannot inflate past the limit. Nothing touches the disk here.
use super::{
    error::{PluginError, Result},
    manifest::{self, Manifest},
};
use std::{
    collections::{BTreeMap, HashSet},
    io::{Cursor, Read},
};
use zip::{CompressionMethod, ZipArchive};

pub const MAX_PACKAGE_BYTES: usize = 20 * 1024 * 1024;
pub const MAX_UNPACKED_BYTES: u64 = 60 * 1024 * 1024;
pub const MAX_FILES: usize = 200;
pub const MAX_DEPTH: usize = 6;

/// A verified package held in memory.
#[derive(Debug, Clone)]
pub struct Package {
    pub manifest: Manifest,
    pub files: BTreeMap<String, Vec<u8>>,
    pub sha256: String,
}

impl Package {
    pub fn wasm(&self) -> &[u8] {
        &self.files[&self.manifest.entry]
    }
}

fn bad(code: &str, message: impl Into<String>) -> PluginError {
    PluginError::package(code, message)
}

/// Relative POSIX path, no `.`/`..`/empty segments, no drive or backslash,
/// limited depth and a conservative character set.
pub fn check_path(name: &str) -> Result<()> {
    let segments: Vec<_> = name.split('/').collect();
    let valid = !name.is_empty()
        && name.len() <= 200
        && !name.starts_with('/')
        && segments.len() <= MAX_DEPTH
        && segments.iter().all(|segment| {
            !segment.is_empty()
                && *segment != "."
                && *segment != ".."
                && segment
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
        });
    if valid {
        Ok(())
    } else {
        Err(bad("PLUGIN_PACKAGE_PATH", format!("Unsafe path in package: {name:?}")))
    }
}

/// Inspect and read a `.cadplugin`. `app_version` gates `minAppVersion`.
pub fn read(bytes: &[u8], app_version: &str) -> Result<Package> {
    use sha2::Digest;
    if bytes.len() > MAX_PACKAGE_BYTES {
        return Err(bad("PLUGIN_PACKAGE_TOO_LARGE", "Package is larger than 20 MB"));
    }
    let mut archive = ZipArchive::new(Cursor::new(bytes))
        .map_err(|error| bad("PLUGIN_PACKAGE_CORRUPT", error.to_string()))?;
    if archive.len() > MAX_FILES {
        return Err(bad("PLUGIN_PACKAGE_TOO_MANY_FILES", "Package has more than 200 entries"));
    }
    // Pass 1: directory only.
    let mut declared_total = 0u64;
    let mut seen = HashSet::new();
    let mut entries = Vec::new();
    for index in 0..archive.len() {
        let entry = archive
            .by_index_raw(index)
            .map_err(|error| bad("PLUGIN_PACKAGE_CORRUPT", error.to_string()))?;
        let name = std::str::from_utf8(entry.name_raw())
            .map_err(|_| bad("PLUGIN_PACKAGE_PATH", "Non UTF-8 path in package"))?
            .to_owned();
        if entry.is_symlink() {
            return Err(bad("PLUGIN_PACKAGE_SYMLINK", format!("Symbolic link in package: {name}")));
        }
        if entry.encrypted() {
            return Err(bad("PLUGIN_PACKAGE_CORRUPT", "Encrypted entries are not allowed"));
        }
        if !matches!(entry.compression(), CompressionMethod::Stored | CompressionMethod::Deflated) {
            return Err(bad("PLUGIN_PACKAGE_CORRUPT", "Only stored or deflated entries are allowed"));
        }
        if entry.is_dir() {
            check_path(name.trim_end_matches('/'))?;
            continue;
        }
        check_path(&name)?;
        // Windows profiles are case-insensitive: `A.json` and `a.json` collide.
        if !seen.insert(name.to_ascii_lowercase()) {
            return Err(bad("PLUGIN_PACKAGE_PATH", format!("Duplicate path in package: {name}")));
        }
        declared_total = declared_total.saturating_add(entry.size());
        if declared_total > MAX_UNPACKED_BYTES {
            return Err(bad("PLUGIN_PACKAGE_TOO_LARGE", "Package unpacks to more than 60 MB"));
        }
        entries.push((index, name, entry.size()));
    }
    // Pass 2: decompress with hard caps.
    let mut files = BTreeMap::new();
    let mut actual_total = 0u64;
    for (index, name, size) in entries {
        let entry = archive
            .by_index(index)
            .map_err(|error| bad("PLUGIN_PACKAGE_CORRUPT", error.to_string()))?;
        let mut data = Vec::with_capacity(size.min(1 << 20) as usize);
        entry
            .take(size + 1)
            .read_to_end(&mut data)
            .map_err(|error| bad("PLUGIN_PACKAGE_CORRUPT", error.to_string()))?;
        if data.len() as u64 != size {
            return Err(bad("PLUGIN_PACKAGE_CORRUPT", format!("Size mismatch for {name}")));
        }
        actual_total += data.len() as u64;
        if actual_total > MAX_UNPACKED_BYTES {
            return Err(bad("PLUGIN_PACKAGE_TOO_LARGE", "Package unpacks to more than 60 MB"));
        }
        files.insert(name, data);
    }
    let manifest_bytes = files
        .get("manifest.json")
        .ok_or_else(|| bad("PLUGIN_PACKAGE_INCOMPLETE", "manifest.json is missing"))?;
    let manifest = manifest::parse(manifest_bytes, app_version)?;
    if !files.contains_key(&manifest.entry) {
        return Err(bad("PLUGIN_PACKAGE_INCOMPLETE", format!("{} is missing", manifest.entry)));
    }
    let icon = manifest.icon.as_deref().filter(|icon| !icon.starts_with("lucide:"));
    if let Some(icon) = icon {
        if !files.contains_key(icon) {
            return Err(bad("PLUGIN_PACKAGE_INCOMPLETE", format!("{icon} is missing")));
        }
    }
    for name in files.keys() {
        let allowed = name == "manifest.json"
            || *name == manifest.entry
            || Some(name.as_str()) == icon
            || name == "icon.png"
            || name.starts_with("assets/")
            || (name.starts_with("locales/") && name.ends_with(".json") && name.matches('/').count() == 1);
        if !allowed {
            return Err(bad("PLUGIN_PACKAGE_UNEXPECTED_FILE", format!("Unexpected file in package: {name}")));
        }
    }
    Ok(Package {
        manifest,
        files,
        sha256: format!("{:x}", sha2::Sha256::digest(bytes)),
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    pub(crate) fn manifest_json() -> serde_json::Value {
        serde_json::json!({
            "id": "test-plugin", "name": "Test", "version": "1.0.0", "description": "Test plugin",
            "categories": ["other"], "author": {"name": "Tester", "github": "tester"},
            "repository": "https://github.com/tester/test-plugin", "license": "MIT",
            "minAppVersion": "1.0.0", "api": "0.1", "entry": "plugin.wasm",
            "permissions": [], "contributes": {"panels": [{"id": "main", "title": "Main", "location": "sidebar"}]}
        })
    }

    pub(crate) fn zip(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (name, data) in files {
            writer
                .start_file(*name, SimpleFileOptions::default().compression_method(CompressionMethod::Deflated))
                .unwrap();
            writer.write_all(data).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    fn code(bytes: &[u8]) -> String {
        read(bytes, "1.3.3").unwrap_err().code
    }

    #[test]
    fn accepts_a_minimal_package() {
        let manifest = manifest_json().to_string();
        let package = read(&zip(&[("manifest.json", manifest.as_bytes()), ("plugin.wasm", b"\0asm"), ("locales/ru.json", b"{}")]), "1.3.3").unwrap();
        assert_eq!(package.wasm(), b"\0asm");
        assert_eq!(package.sha256.len(), 64);
    }

    #[test]
    fn rejects_traversal_absolute_and_odd_paths() {
        let manifest = manifest_json().to_string();
        for name in ["../evil.wasm", "assets/../../x", "/abs.json", "C:/x", "assets\\x", "assets/./x", "a/b/c/d/e/f/g"] {
            let bytes = zip(&[("manifest.json", manifest.as_bytes()), ("plugin.wasm", b"\0asm"), (name, b"x")]);
            assert_eq!(code(&bytes), "PLUGIN_PACKAGE_PATH", "{name}");
        }
    }

    #[test]
    fn rejects_corrupt_incomplete_and_unexpected_content() {
        assert_eq!(code(b"PK\x03\x04 not really a zip"), "PLUGIN_PACKAGE_CORRUPT");
        assert_eq!(code(&zip(&[("plugin.wasm", b"\0asm")])), "PLUGIN_PACKAGE_INCOMPLETE");
        let manifest = manifest_json().to_string();
        assert_eq!(code(&zip(&[("manifest.json", manifest.as_bytes())])), "PLUGIN_PACKAGE_INCOMPLETE");
        let extra = zip(&[("manifest.json", manifest.as_bytes()), ("plugin.wasm", b"\0asm"), ("run.exe", b"MZ")]);
        assert_eq!(code(&extra), "PLUGIN_PACKAGE_UNEXPECTED_FILE");
        let mut truncated = zip(&[("manifest.json", manifest.as_bytes()), ("plugin.wasm", b"\0asm")]);
        truncated.truncate(truncated.len() / 2);
        assert_eq!(code(&truncated), "PLUGIN_PACKAGE_CORRUPT");
    }

    #[test]
    fn enforces_file_count_and_size_limits() {
        let names: Vec<String> = (0..=MAX_FILES).map(|i| format!("assets/{i}.bin")).collect();
        let files: Vec<(&str, &[u8])> = names.iter().map(|n| (n.as_str(), &b"x"[..])).collect();
        assert_eq!(code(&zip(&files)), "PLUGIN_PACKAGE_TOO_MANY_FILES");
        // 61 MB of zeros deflates to a tiny archive: a classic bomb.
        let zeros = vec![0u8; (MAX_UNPACKED_BYTES + 1024 * 1024) as usize];
        assert_eq!(code(&zip(&[("assets/zeros.bin", &zeros)])), "PLUGIN_PACKAGE_TOO_LARGE");
        assert_eq!(code(&vec![0u8; MAX_PACKAGE_BYTES + 1]), "PLUGIN_PACKAGE_TOO_LARGE");
    }

    #[test]
    fn rejects_symlinks() {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        writer.add_symlink("assets/link", "../../etc/passwd", SimpleFileOptions::default()).unwrap();
        let bytes = writer.finish().unwrap().into_inner();
        assert_eq!(code(&bytes), "PLUGIN_PACKAGE_SYMLINK");
    }
}
