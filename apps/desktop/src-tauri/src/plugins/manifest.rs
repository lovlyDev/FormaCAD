//! `manifest.json`: JSON Schema validation (the published schema itself is
//! embedded) followed by host-only semantic checks.
use super::{
    error::{PluginError, Result},
    forms::FormField,
    permissions::{self, Permission, PermissionSet},
};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

const SCHEMA: &str = include_str!("../../../../../docs/schemas/plugin-manifest.schema.json");
/// Contract version implemented by this host.
pub const HOST_API: (u64, u64) = (0, 2);
pub const MAX_MANIFEST_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub about: Option<String>,
    #[serde(default)]
    pub highlights: Vec<Highlight>,
    pub categories: Vec<String>,
    pub author: Author,
    pub repository: String,
    pub license: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_color: Option<String>,
    pub min_app_version: String,
    pub api: String,
    pub entry: String,
    pub permissions: Vec<String>,
    pub contributes: Contributes,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub links: Option<Links>,
    #[serde(default)]
    pub locales: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Highlight {
    pub icon: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Author {
    pub name: String,
    pub github: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Links {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub funding: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issues: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Contributes {
    #[serde(default)]
    pub features: Vec<Contribution>,
    #[serde(default)]
    pub commands: Vec<Contribution>,
    #[serde(default)]
    pub importers: Vec<FormatContribution>,
    #[serde(default)]
    pub exporters: Vec<FormatContribution>,
    #[serde(default)]
    pub panels: Vec<Panel>,
    #[serde(default)]
    pub settings: Vec<FormField>,
}

/// A feature or a command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Contribution {
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Commands only: `always`, `model` or `selection`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
    #[serde(default)]
    pub params: Vec<FormField>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FormatContribution {
    pub id: String,
    pub title: String,
    pub extensions: Vec<String>,
    #[serde(default)]
    pub params: Vec<FormField>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Panel {
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    pub location: String,
}

fn validator() -> &'static jsonschema::Validator {
    static VALIDATOR: OnceLock<jsonschema::Validator> = OnceLock::new();
    VALIDATOR.get_or_init(|| {
        let schema: serde_json::Value =
            serde_json::from_str(SCHEMA).expect("embedded manifest schema is JSON");
        jsonschema::validator_for(&schema).expect("embedded manifest schema compiles")
    })
}

/// Parse and fully validate a manifest. `app_version` is the running CAD version.
pub fn parse(bytes: &[u8], app_version: &str) -> Result<Manifest> {
    if bytes.len() > MAX_MANIFEST_BYTES {
        return Err(invalid("manifest.json is larger than 256 KB"));
    }
    let text = std::str::from_utf8(bytes).map_err(|_| invalid("manifest.json is not UTF-8"))?;
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|error| invalid(format!("manifest.json: {error}")))?;
    let errors: Vec<String> = validator()
        .iter_errors(&value)
        .take(5)
        .map(|error| format!("{} {}", error.instance_path(), error))
        .collect();
    if !errors.is_empty() {
        return Err(invalid(errors.join("; ")));
    }
    let manifest: Manifest =
        serde_json::from_value(value).map_err(|error| invalid(error.to_string()))?;
    manifest.check(app_version)?;
    Ok(manifest)
}

fn invalid(message: impl Into<String>) -> PluginError {
    PluginError::package("PLUGIN_MANIFEST_INVALID", message)
}

impl Manifest {
    pub fn permission_set(&self) -> Result<PermissionSet> {
        permissions::parse_all(&self.permissions)
    }

    /// Declared contract version as `(MAJOR, MINOR)`.
    pub fn api_version(&self) -> Result<(u64, u64)> {
        self.api
            .split_once('.')
            .and_then(|(major, minor)| {
                Some((major.parse::<u64>().ok()?, minor.parse::<u64>().ok()?))
            })
            .ok_or_else(|| invalid("api must be MAJOR.MINOR"))
    }

    fn check(&self, app_version: &str) -> Result<()> {
        let api = self.api_version()?;
        if api.0 != HOST_API.0 || api.1 > HOST_API.1 {
            return Err(PluginError::package(
                "PLUGIN_API_UNSUPPORTED",
                format!(
                    "Plugin needs plugin-api {}, this FormaCAD provides {}.{}",
                    self.api, HOST_API.0, HOST_API.1
                ),
            ));
        }
        let required = semver::Version::parse(&self.min_app_version)
            .map_err(|_| invalid("minAppVersion is not a version"))?;
        let running = semver::Version::parse(app_version)
            .map_err(|_| PluginError::host("PLUGIN_HOST", "Application version is not SemVer"))?;
        if running < required {
            return Err(PluginError::package(
                "PLUGIN_APP_TOO_OLD",
                format!("Plugin needs FormaCAD {required} or newer"),
            ));
        }
        let granted = self.permission_set()?;
        let c = &self.contributes;
        let unique = |ids: Vec<&str>, what: &str| -> Result<()> {
            let mut seen = std::collections::HashSet::new();
            match ids.into_iter().find(|id| !seen.insert(*id)) {
                Some(id) => Err(invalid(format!("Duplicate {what} id {id}"))),
                None => Ok(()),
            }
        };
        unique(
            c.features.iter().map(|f| f.id.as_str()).collect(),
            "feature",
        )?;
        unique(
            c.commands.iter().map(|f| f.id.as_str()).collect(),
            "command",
        )?;
        unique(
            c.importers.iter().map(|f| f.id.as_str()).collect(),
            "importer",
        )?;
        unique(
            c.exporters.iter().map(|f| f.id.as_str()).collect(),
            "exporter",
        )?;
        unique(c.panels.iter().map(|f| f.id.as_str()).collect(), "panel")?;
        for form in c
            .features
            .iter()
            .chain(&c.commands)
            .map(|item| &item.params)
            .chain(
                c.importers
                    .iter()
                    .chain(&c.exporters)
                    .map(|item| &item.params),
            )
            .chain(std::iter::once(&c.settings))
        {
            super::forms::check_form(form).map_err(invalid)?;
        }
        let need = |condition: bool, permission: Permission, what: &str| -> Result<()> {
            if condition && !granted.contains(&permission) {
                return Err(PluginError::package(
                    "PLUGIN_PERMISSION_MISSING",
                    format!("{what} require the {} permission", permission.as_string()),
                ));
            }
            Ok(())
        };
        need(!c.features.is_empty(), Permission::ModelWrite, "features")?;
        // Commands without `model.write` may only return an empty plan
        // (checked in `PluginService::plan`), e.g. to record a video.
        need(!c.importers.is_empty(), Permission::FileImport, "importers")?;
        need(!c.exporters.is_empty(), Permission::FileExport, "exporters")?;
        need(!c.exporters.is_empty(), Permission::ModelRead, "exporters")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &str = include_str!("../../../../../docs/schemas/plugin-manifest.example.json");

    #[test]
    fn published_example_manifest_matches_the_schema() {
        let manifest = parse(EXAMPLE.as_bytes(), "2.12.0").unwrap();
        assert_eq!(manifest.id, "spur-gear");
        assert_eq!(manifest.contributes.features[0].params.len(), 5);
    }

    #[test]
    fn rejects_schema_api_version_and_permission_violations() {
        let mut value: serde_json::Value = serde_json::from_str(EXAMPLE).unwrap();
        let check = |value: &serde_json::Value, version: &str| {
            parse(value.to_string().as_bytes(), version)
                .unwrap_err()
                .code
        };
        assert_eq!(check(&value, "1.3.3"), "PLUGIN_APP_TOO_OLD");
        value["api"] = "0.3".into();
        assert_eq!(check(&value, "2.12.0"), "PLUGIN_API_UNSUPPORTED");
        value["api"] = "0.2".into();
        value["permissions"] = serde_json::json!(["model.write", "viewport.capture"]);
        assert!(parse(value.to_string().as_bytes(), "2.12.0").is_ok());
        value["api"] = "0.1".into();
        value["permissions"] = serde_json::json!(["model.read"]);
        assert_eq!(check(&value, "2.12.0"), "PLUGIN_PERMISSION_MISSING");
        value["permissions"] = serde_json::json!(["model.write", "fs.write"]);
        assert_eq!(check(&value, "2.12.0"), "PLUGIN_MANIFEST_INVALID");
        value["permissions"] = serde_json::json!(["model.write"]);
        value["unexpected"] = true.into();
        assert_eq!(check(&value, "2.12.0"), "PLUGIN_MANIFEST_INVALID");
        assert_eq!(
            parse(b"\xff", "2.12.0").unwrap_err().code,
            "PLUGIN_MANIFEST_INVALID"
        );
    }
}
