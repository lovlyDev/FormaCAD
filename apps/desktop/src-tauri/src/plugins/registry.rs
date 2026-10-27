//! Installed plugins on disk:
//!
//! ```text
//! <profile>/plugins/registry.json          installed versions, active version, consent, settings
//! <profile>/plugins/<id>/<version>/…       unpacked package (immutable once written)
//! <profile>/plugins/data/<id>/storage.json storage permission data
//! <profile>/plugins/journal.jsonl          local journal
//! ```
use super::{
    error::{PluginError, Result},
    manifest::{self, Manifest},
    package::Package,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Registry {
    #[serde(default)]
    pub plugins: BTreeMap<String, Installed>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Installed {
    /// Every unpacked version, SemVer ascending.
    pub versions: Vec<InstalledVersion>,
    pub active: String,
    pub enabled: bool,
    /// Permissions the user agreed to for the active version.
    pub accepted_permissions: Vec<String>,
    #[serde(default)]
    pub settings: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledVersion {
    pub version: String,
    pub sha256: String,
    /// `file` or `catalog`.
    pub source: String,
    pub installed_at: String,
}

pub struct Store {
    pub root: PathBuf,
}

impl Store {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn registry_path(&self) -> PathBuf {
        self.root.join("registry.json")
    }

    pub fn version_dir(&self, id: &str, version: &str) -> PathBuf {
        self.root.join(id).join(version)
    }

    pub fn data_dir(&self, id: &str) -> PathBuf {
        self.root.join("data").join(id)
    }

    pub fn load(&self) -> Result<Registry> {
        match std::fs::read(self.registry_path()) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|error| PluginError::host("PLUGIN_REGISTRY_CORRUPT", error.to_string())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Registry::default()),
            Err(error) => Err(error.into()),
        }
    }

    pub fn save(&self, registry: &Registry) -> Result<()> {
        std::fs::create_dir_all(&self.root)?;
        write_atomic(&self.registry_path(), &serde_json::to_vec_pretty(registry)?)
    }

    /// Read the manifest of an installed version (re-validated on every load).
    pub fn manifest(&self, id: &str, version: &str, app_version: &str) -> Result<Manifest> {
        let bytes = std::fs::read(self.version_dir(id, version).join("manifest.json"))?;
        manifest::parse(&bytes, app_version).map_err(|error| error.for_plugin(id))
    }

    /// Unpack into `<id>/<version>/` via a sibling staging directory and a
    /// rename, so a crash never leaves a half-written version that looks valid.
    pub fn unpack(&self, package: &Package) -> Result<()> {
        let id = &package.manifest.id;
        let target = self.version_dir(id, &package.manifest.version);
        if target.exists() {
            return Ok(());
        }
        let staging = self
            .root
            .join(id)
            .join(format!(".staging-{}", uuid::Uuid::new_v4()));
        let result = (|| {
            for (name, data) in &package.files {
                // Paths were validated by package::check_path; join stays inside.
                let path = staging.join(Path::new(name));
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&path, data)?;
            }
            std::fs::rename(&staging, &target)
        })();
        if result.is_err() {
            let _ = std::fs::remove_dir_all(&staging);
        }
        result.map_err(Into::into)
    }

    pub fn remove_plugin(&self, id: &str) -> Result<()> {
        for dir in [self.root.join(id), self.data_dir(id)] {
            match std::fs::remove_dir_all(&dir) {
                Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                    return Err(error.into())
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub fn remove_version(&self, id: &str, version: &str) -> Result<()> {
        std::fs::remove_dir_all(self.version_dir(id, version)).map_err(Into::into)
    }
}

pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let temp = path.with_extension(format!("tmp-{}", uuid::Uuid::new_v4()));
    std::fs::write(&temp, bytes)?;
    std::fs::rename(&temp, path).map_err(|error| {
        let _ = std::fs::remove_file(&temp);
        error.into()
    })
}

pub fn sort_versions(versions: &mut [InstalledVersion]) {
    versions.sort_by(|a, b| {
        match (
            semver::Version::parse(&a.version),
            semver::Version::parse(&b.version),
        ) {
            (Ok(a), Ok(b)) => a.cmp(&b),
            _ => a.version.cmp(&b.version),
        }
    });
}
