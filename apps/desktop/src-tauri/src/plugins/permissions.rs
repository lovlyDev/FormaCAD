//! Manifest permissions and their binding to `formacad:plugin@0.1` interfaces.
use super::error::{PluginError, Result};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Permission {
    ModelRead,
    ModelWrite,
    SelectionRead,
    FileImport,
    FileExport,
    Storage,
    Notifications,
    Network(String),
}

impl Permission {
    pub fn parse(raw: &str) -> Result<Self> {
        Ok(match raw {
            "model.read" => Self::ModelRead,
            "model.write" => Self::ModelWrite,
            "selection.read" => Self::SelectionRead,
            "file.import" => Self::FileImport,
            "file.export" => Self::FileExport,
            "storage" => Self::Storage,
            "notifications" => Self::Notifications,
            _ => match raw.strip_prefix("network:") {
                Some(host) if valid_host(host) => Self::Network(host.to_owned()),
                _ => {
                    return Err(PluginError::package(
                        "PLUGIN_PERMISSION_UNKNOWN",
                        format!("Unknown permission {raw}"),
                    ))
                }
            },
        })
    }

    pub fn as_string(&self) -> String {
        match self {
            Self::ModelRead => "model.read".into(),
            Self::ModelWrite => "model.write".into(),
            Self::SelectionRead => "selection.read".into(),
            Self::FileImport => "file.import".into(),
            Self::FileExport => "file.export".into(),
            Self::Storage => "storage".into(),
            Self::Notifications => "notifications".into(),
            Self::Network(host) => format!("network:{host}"),
        }
    }
}

/// Hostname only: lowercase labels, at least one dot, no ports, no IP literals
/// that would let a plugin reach the local network by number.
fn valid_host(host: &str) -> bool {
    let labels: Vec<_> = host.split('.').collect();
    host.len() <= 253
        && labels.len() >= 2
        && labels.iter().all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        })
        && !labels
            .last()
            .is_some_and(|tld| tld.bytes().all(|b| b.is_ascii_digit()))
        && host != "localhost"
        && !host.ends_with(".localhost")
}

pub type PermissionSet = BTreeSet<Permission>;

pub fn parse_all(raw: &[String]) -> Result<PermissionSet> {
    raw.iter().map(|value| Permission::parse(value)).collect()
}

pub fn to_strings(set: &PermissionSet) -> Vec<String> {
    set.iter().map(Permission::as_string).collect()
}

/// WIT package all host imports and plugin exports must belong to.
pub const WIT_PACKAGE: &str = "formacad:plugin";
pub const WIT_VERSION: &str = "0.1.0";

/// The import is not part of the host contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnknownImport;

/// Which permission an imported interface needs. `Ok(None)` means "always
/// available".
pub fn import_requirement(interface: &str) -> std::result::Result<Option<Requirement>, UnknownImport> {
    Ok(match interface {
        "types" | "log" | "assets" => None,
        "model" => Some(Requirement::Exact(Permission::ModelRead)),
        "selection" => Some(Requirement::Exact(Permission::SelectionRead)),
        "storage" => Some(Requirement::Exact(Permission::Storage)),
        "notifications" => Some(Requirement::Exact(Permission::Notifications)),
        "network" => Some(Requirement::AnyNetwork),
        _ => return Err(UnknownImport),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Requirement {
    Exact(Permission),
    AnyNetwork,
}

impl Requirement {
    pub fn satisfied_by(&self, granted: &PermissionSet) -> bool {
        match self {
            Self::Exact(permission) => granted.contains(permission),
            Self::AnyNetwork => granted.iter().any(|p| matches!(p, Permission::Network(_))),
        }
    }
    pub fn describe(&self) -> String {
        match self {
            Self::Exact(permission) => permission.as_string(),
            Self::AnyNetwork => "network:<host>".into(),
        }
    }
}

/// Permissions that are new in `next` compared with what the user accepted.
pub fn added(accepted: &PermissionSet, next: &PermissionSet) -> PermissionSet {
    next.difference(accepted).cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_permissions_and_rejects_unsafe_hosts() {
        assert_eq!(Permission::parse("model.read").unwrap(), Permission::ModelRead);
        assert_eq!(
            Permission::parse("network:api.example.com").unwrap(),
            Permission::Network("api.example.com".into())
        );
        for bad in [
            "network:localhost",
            "network:127.0.0.1",
            "network:example",
            "network:Example.com",
            "network:a.localhost",
            "network:-a.com",
            "fs.write",
        ] {
            assert!(Permission::parse(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn added_permissions_are_a_set_difference() {
        let old = parse_all(&["model.read".into()]).unwrap();
        let new = parse_all(&["model.read".into(), "network:a.example.com".into()]).unwrap();
        assert_eq!(to_strings(&added(&old, &new)), vec!["network:a.example.com"]);
        assert!(added(&new, &old).is_empty());
    }

    #[test]
    fn imports_map_to_one_permission_or_are_rejected() {
        assert_eq!(import_requirement("log"), Ok(None));
        assert_eq!(
            import_requirement("model"),
            Ok(Some(Requirement::Exact(Permission::ModelRead)))
        );
        assert!(import_requirement("filesystem").is_err());
    }
}
