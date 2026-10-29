//! Static checks of a plugin component before installation: it must be a
//! WebAssembly component, import only `formacad:plugin` interfaces of the
//! declared plugin-api version, and every import must be covered by a declared
//! permission.
use super::{
    error::{PluginError, Result},
    manifest::Manifest,
    permissions::{import_requirement, interface_since, PermissionSet, WIT_PACKAGE, WIT_VERSION},
};
use wasmtime::{component::Component, Engine};

/// Interface name inside the contract: `formacad:plugin/model@0.1.0` → `model`.
pub fn contract_interface(name: &str) -> Option<&str> {
    name.strip_prefix(WIT_PACKAGE)?
        .strip_prefix('/')?
        .strip_suffix(WIT_VERSION)?
        .strip_suffix('@')
}

pub fn export_name(interface: &str) -> String {
    format!("{WIT_PACKAGE}/{interface}@{WIT_VERSION}")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Surface {
    pub imports: Vec<String>,
    pub exports: Vec<String>,
}

/// Compile (validates the binary) and inspect the component.
pub fn inspect(engine: &Engine, wasm: &[u8]) -> Result<(Component, Surface)> {
    if wasmparser_is_core_module(wasm) {
        return Err(PluginError::package(
            "PLUGIN_WASM_NOT_COMPONENT",
            "plugin.wasm is a core module; plugin-api needs a WebAssembly component",
        ));
    }
    let component = Component::new(engine, wasm)
        .map_err(|error| PluginError::package("PLUGIN_WASM_INVALID", format!("{error:#}")))?;
    let ty = component.component_type();
    let surface = Surface {
        imports: ty
            .imports(engine)
            .map(|(name, _)| name.to_owned())
            .collect(),
        exports: ty
            .exports(engine)
            .map(|(name, _)| name.to_owned())
            .collect(),
    };
    Ok((component, surface))
}

/// Core modules start with version 1 / layer 0; components use layer 1.
fn wasmparser_is_core_module(wasm: &[u8]) -> bool {
    wasm.len() >= 8 && wasm[..4] == *b"\0asm" && wasm[4..8] == [1, 0, 0, 0]
}

/// Check imports against permissions and exports against contributions.
pub fn check(surface: &Surface, manifest: &Manifest, granted: &PermissionSet) -> Result<()> {
    let api = manifest.api_version()?;
    for import in &surface.imports {
        let interface = contract_interface(import).ok_or_else(|| forbidden(import))?;
        let requirement = import_requirement(interface).map_err(|_| forbidden(import))?;
        let since = interface_since(interface);
        if api < since {
            return Err(PluginError::package(
                "PLUGIN_IMPORT_API_TOO_OLD",
                format!(
                    "Import {import} needs \"api\": \"{}.{}\" in the manifest",
                    since.0, since.1
                ),
            ));
        }
        if let Some(requirement) = requirement {
            if !requirement.satisfied_by(granted) {
                return Err(PluginError::package(
                    "PLUGIN_IMPORT_NOT_PERMITTED",
                    format!("Import {import} needs the {} permission, which the manifest does not declare", requirement.describe()),
                ));
            }
        }
    }
    let c = &manifest.contributes;
    for (present, interface) in [
        (!c.features.is_empty(), "feature"),
        (!c.commands.is_empty(), "command"),
        (!c.importers.is_empty(), "importer"),
        (!c.exporters.is_empty(), "exporter"),
        (!c.panels.is_empty(), "panel"),
    ] {
        if present && !surface.exports.contains(&export_name(interface)) {
            return Err(PluginError::package(
                "PLUGIN_EXPORT_MISSING",
                format!(
                    "Manifest contributes {interface}s but the component does not export {}",
                    export_name(interface)
                ),
            ));
        }
    }
    Ok(())
}

fn forbidden(import: &str) -> PluginError {
    PluginError::package(
        "PLUGIN_IMPORT_FORBIDDEN",
        format!("Import {import} is not part of {WIT_PACKAGE}@{WIT_VERSION} (no WASI or other host access)"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_interface_names_are_exact() {
        assert_eq!(
            contract_interface("formacad:plugin/model@0.1.0"),
            Some("model")
        );
        assert_eq!(contract_interface("formacad:plugin/model@0.2.0"), None);
        assert_eq!(contract_interface("wasi:filesystem/types@0.2.0"), None);
        assert_eq!(contract_interface("formacad:pluginx/model@0.1.0"), None);
    }

    #[test]
    fn core_modules_are_refused_before_compilation() {
        let engine = Engine::default();
        let module = wat::parse_str("(module)").unwrap();
        assert_eq!(
            inspect(&engine, &module).unwrap_err().code,
            "PLUGIN_WASM_NOT_COMPONENT"
        );
        assert_eq!(
            inspect(&engine, b"garbage").unwrap_err().code,
            "PLUGIN_WASM_INVALID"
        );
    }
}
