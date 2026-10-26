//! Plugin lifecycle: two-step install (verify → user confirms), versions,
//! enable/disable, removal, settings and plugin calls. Synchronous; IPC runs
//! it on blocking threads.
use super::{
    error::{PluginError, Result},
    forms,
    journal::{Entry, Journal},
    manifest::Manifest,
    package::{self, Package},
    permissions::{self, PermissionSet},
    registry::{sort_versions, Installed, InstalledVersion, Store},
    runtime::{self, CallReport, CallSetup, Limits, Runtime},
    wasm_check,
};
use crate::cad_ir::Command;
use serde::Serialize;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::Mutex,
    time::{Duration, Instant},
};

pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const PENDING_TTL: Duration = Duration::from_secs(15 * 60);
const MAX_PENDING: usize = 4;

/// Where a package came from; catalog installs carry the verified page URL.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Source {
    File,
    Catalog { page_url: String },
}

impl Source {
    fn name(&self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Catalog { .. } => "catalog",
        }
    }
}

struct Pending {
    package: Package,
    source: Source,
    created: Instant,
}

/// What the confirmation window shows. Nothing is installed yet.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallPreview {
    pub token: String,
    pub manifest: Manifest,
    pub permissions: Vec<String>,
    /// Permissions not covered by the user's earlier consent (all of them for a new plugin).
    pub added_permissions: Vec<String>,
    pub installed_version: Option<String>,
    pub is_update: bool,
    pub is_downgrade: bool,
    pub already_installed: bool,
    pub source: Source,
    pub sha256: String,
    pub translations: Translations,
}

/// Plugin strings by language from `locales/<lang>.json`, used by the UI to
/// resolve `%key%` manifest strings. Invalid files are ignored.
pub type Translations = std::collections::BTreeMap<String, std::collections::BTreeMap<String, String>>;

fn translations(manifest: &Manifest, read: impl Fn(&str) -> Option<Vec<u8>>) -> Translations {
    manifest
        .locales
        .iter()
        .filter_map(|lang| {
            let bytes = read(&format!("locales/{lang}.json")).filter(|b| b.len() <= 256 * 1024)?;
            let strings = serde_json::from_slice(&bytes).ok()?;
            Some((lang.clone(), strings))
        })
        .collect()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginSummary {
    pub manifest: Manifest,
    pub versions: Vec<InstalledVersion>,
    pub active: String,
    pub enabled: bool,
    pub accepted_permissions: Vec<String>,
    pub settings: serde_json::Map<String, serde_json::Value>,
    pub translations: Translations,
    /// Set when the active version could not be loaded (e.g. newer api).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub problem: Option<String>,
}

/// Inputs from the open project for a plugin call.
#[derive(Debug, Clone, Default)]
pub struct CallInput {
    pub locale: String,
    pub revision_id: Option<String>,
    pub document: Option<String>,
    pub selection: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub plugin_id: String,
    pub plugin_name: String,
    pub version: String,
    pub title: String,
    pub message: Option<String>,
    pub commands: Vec<Command>,
    pub report: CallReport,
}

pub struct PluginService {
    pub store: Store,
    pub journal: Journal,
    pending: Mutex<HashMap<String, Pending>>,
    registry_lock: Mutex<()>,
    limits: Limits,
}

impl PluginService {
    pub fn new(root: PathBuf) -> Self {
        Self {
            journal: Journal::new(&root),
            store: Store::new(root),
            pending: Mutex::default(),
            registry_lock: Mutex::default(),
            limits: Limits::default(),
        }
    }

    #[cfg(test)]
    pub fn with_limits(mut self, limits: Limits) -> Self {
        self.limits = limits;
        self
    }

    fn runtime(&self) -> &'static Runtime {
        Runtime::global()
    }

    /// Step 1: verify everything that can be verified without the user.
    pub fn prepare(&self, bytes: &[u8], source: Source) -> Result<InstallPreview> {
        let package = package::read(bytes, APP_VERSION)?;
        let declared = package.manifest.permission_set()?;
        let (_, surface) = wasm_check::inspect(self.runtime().engine(), package.wasm())?;
        wasm_check::check(&surface, &package.manifest, &declared)?;
        let registry = self.store.load()?;
        let existing = registry.plugins.get(&package.manifest.id);
        if let Some(installed) = existing {
            if let Some(same) = installed.versions.iter().find(|v| v.version == package.manifest.version) {
                if same.sha256 != package.sha256 {
                    return Err(PluginError::package(
                        "PLUGIN_VERSION_CONFLICT",
                        "This version is already installed with different contents; versions are immutable",
                    ));
                }
            }
        }
        let accepted = match existing {
            Some(installed) => permissions::parse_all(&installed.accepted_permissions).unwrap_or_default(),
            None => PermissionSet::new(),
        };
        let next = semver::Version::parse(&package.manifest.version).ok();
        let active = existing.and_then(|i| semver::Version::parse(&i.active).ok());
        let preview = InstallPreview {
            token: uuid::Uuid::new_v4().to_string(),
            permissions: permissions::to_strings(&declared),
            added_permissions: permissions::to_strings(&permissions::added(&accepted, &declared)),
            installed_version: existing.map(|i| i.active.clone()),
            is_update: existing.is_some() && next > active,
            is_downgrade: existing.is_some() && next < active,
            already_installed: existing.is_some_and(|i| i.versions.iter().any(|v| v.version == package.manifest.version)),
            source: source.clone(),
            sha256: package.sha256.clone(),
            translations: translations(&package.manifest, |name| package.files.get(name).cloned()),
            manifest: package.manifest.clone(),
        };
        let mut pending = self.pending.lock().unwrap_or_else(|p| p.into_inner());
        pending.retain(|_, p| p.created.elapsed() < PENDING_TTL);
        while pending.len() >= MAX_PENDING {
            let oldest = pending.iter().min_by_key(|(_, p)| p.created).map(|(k, _)| k.clone());
            if let Some(oldest) = oldest {
                pending.remove(&oldest);
            }
        }
        pending.insert(preview.token.clone(), Pending { package, source, created: Instant::now() });
        Ok(preview)
    }

    pub fn cancel(&self, token: &str) {
        self.pending.lock().unwrap_or_else(|p| p.into_inner()).remove(token);
    }

    /// Step 2: the user pressed "Install". Consent covers exactly the
    /// permissions shown in the preview.
    pub fn confirm(&self, token: &str) -> Result<PluginSummary> {
        let pending = self
            .pending
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .remove(token)
            .filter(|p| p.created.elapsed() < PENDING_TTL)
            .ok_or_else(|| PluginError::host("PLUGIN_INSTALL_EXPIRED", "The install request expired; start again"))?;
        let manifest = &pending.package.manifest;
        let _guard = self.registry_lock.lock().unwrap_or_else(|p| p.into_inner());
        self.store.unpack(&pending.package)?;
        let mut registry = self.store.load()?;
        let record = InstalledVersion {
            version: manifest.version.clone(),
            sha256: pending.package.sha256.clone(),
            source: pending.source.name().into(),
            installed_at: chrono::Utc::now().to_rfc3339(),
        };
        let updating = registry.plugins.contains_key(&manifest.id);
        let entry = registry.plugins.entry(manifest.id.clone()).or_insert_with(|| Installed {
            versions: vec![],
            active: manifest.version.clone(),
            enabled: true,
            accepted_permissions: vec![],
            settings: Default::default(),
        });
        if !entry.versions.iter().any(|v| v.version == record.version) {
            entry.versions.push(record);
            sort_versions(&mut entry.versions);
        }
        entry.active = manifest.version.clone();
        entry.accepted_permissions = manifest.permissions.clone();
        entry.settings = forms::resolve_values(&manifest.contributes.settings, &serde_json::Value::Object(entry.settings.clone()))
            .or_else(|_| forms::resolve_values(&manifest.contributes.settings, &serde_json::Value::Null))
            .map_err(|e| PluginError::package("PLUGIN_MANIFEST_INVALID", e))?;
        self.store.save(&registry)?;
        self.journal.append(&Entry::new(
            &manifest.id,
            Some(&manifest.version),
            if updating { "update" } else { "install" },
            true,
            format!("source={} sha256={} permissions={}", pending.source.name(), pending.package.sha256, manifest.permissions.join(",")),
        ));
        self.summary(&manifest.id)
    }

    pub fn list(&self) -> Result<Vec<PluginSummary>> {
        let registry = self.store.load()?;
        registry.plugins.keys().map(|id| self.summary(id)).collect()
    }

    pub fn summary(&self, id: &str) -> Result<PluginSummary> {
        let registry = self.store.load()?;
        let installed = registry.plugins.get(id).ok_or_else(|| not_installed(id))?;
        let (manifest, problem) = match self.store.manifest(id, &installed.active, APP_VERSION) {
            Ok(manifest) => (manifest, None),
            Err(error) => {
                // Still list it so the user can remove it.
                let raw = std::fs::read(self.store.version_dir(id, &installed.active).join("manifest.json"))
                    .ok()
                    .and_then(|bytes| serde_json::from_slice::<Manifest>(&bytes).ok())
                    .ok_or_else(|| error.clone())?;
                (raw, Some(error.code))
            }
        };
        let dir = self.store.version_dir(id, &installed.active);
        Ok(PluginSummary {
            translations: translations(&manifest, |name| std::fs::read(dir.join(name)).ok()),
            manifest,
            versions: installed.versions.clone(),
            active: installed.active.clone(),
            enabled: installed.enabled,
            accepted_permissions: installed.accepted_permissions.clone(),
            settings: installed.settings.clone(),
            problem,
        })
    }

    fn update(&self, id: &str, change: impl FnOnce(&mut Installed) -> Result<String>) -> Result<PluginSummary> {
        let _guard = self.registry_lock.lock().unwrap_or_else(|p| p.into_inner());
        let mut registry = self.store.load()?;
        let installed = registry.plugins.get_mut(id).ok_or_else(|| not_installed(id))?;
        let event = change(installed)?;
        let version = installed.active.clone();
        self.store.save(&registry)?;
        self.journal.append(&Entry::new(id, Some(&version), &event, true, ""));
        self.summary(id)
    }

    pub fn set_enabled(&self, id: &str, enabled: bool) -> Result<PluginSummary> {
        self.update(id, |installed| {
            installed.enabled = enabled;
            Ok(if enabled { "enable" } else { "disable" }.into())
        })
    }

    /// Switch to another installed version. `accepted` must cover every
    /// permission of that version: the UI asks again when it adds any.
    pub fn set_active(&self, id: &str, version: &str, accepted: &[String]) -> Result<PluginSummary> {
        let manifest = self.store.manifest(id, version, APP_VERSION)?;
        let declared = manifest.permission_set()?;
        let consent = permissions::parse_all(accepted)?;
        if !permissions::added(&consent, &declared).is_empty() {
            return Err(PluginError::host("PLUGIN_CONSENT_REQUIRED", "The version needs permissions that were not accepted"));
        }
        self.update(id, |installed| {
            if !installed.versions.iter().any(|v| v.version == version) {
                return Err(not_installed(id));
            }
            installed.active = version.to_owned();
            installed.accepted_permissions = manifest.permissions.clone();
            Ok("activate".into())
        })
    }

    pub fn set_settings(&self, id: &str, values: &serde_json::Value) -> Result<PluginSummary> {
        let installed = self.summary(id)?;
        let resolved = forms::resolve_values(&installed.manifest.contributes.settings, values)
            .map_err(|e| PluginError::host("PLUGIN_PARAMS_INVALID", e).for_plugin(id))?;
        self.update(id, |entry| {
            entry.settings = resolved;
            Ok("settings".into())
        })
    }

    /// Remove one version, or the whole plugin with its data when `version` is None
    /// or it is the last version.
    pub fn uninstall(&self, id: &str, version: Option<&str>) -> Result<()> {
        let _guard = self.registry_lock.lock().unwrap_or_else(|p| p.into_inner());
        let mut registry = self.store.load()?;
        let installed = registry.plugins.get_mut(id).ok_or_else(|| not_installed(id))?;
        let whole = match version {
            None => true,
            Some(version) => {
                if version == installed.active {
                    return Err(PluginError::host("PLUGIN_VERSION_ACTIVE", "Switch to another version before removing the active one"));
                }
                installed.versions.retain(|v| v.version != version);
                self.store.remove_version(id, version)?;
                installed.versions.is_empty()
            }
        };
        if whole {
            registry.plugins.remove(id);
            self.store.remove_plugin(id)?;
        }
        self.store.save(&registry)?;
        self.journal.append(&Entry::new(id, version, "uninstall", true, ""));
        Ok(())
    }

    /// Resolve an enabled plugin's active version for a call.
    fn loaded(&self, id: &str) -> Result<(Installed, Manifest, PermissionSet)> {
        let registry = self.store.load()?;
        let installed = registry.plugins.get(id).cloned().ok_or_else(|| not_installed(id))?;
        if !installed.enabled {
            return Err(PluginError::host("PLUGIN_DISABLED", "The plugin is disabled").for_plugin(id));
        }
        let manifest = self.store.manifest(id, &installed.active, APP_VERSION)?;
        let accepted = permissions::parse_all(&installed.accepted_permissions)?;
        let granted: PermissionSet = manifest.permission_set()?.intersection(&accepted).cloned().collect();
        Ok((installed, manifest, granted))
    }

    fn setup(&self, installed: &Installed, manifest: &Manifest, granted: PermissionSet, input: &CallInput) -> CallSetup {
        let dir = self.store.version_dir(&manifest.id, &installed.active);
        let sha256 = installed
            .versions
            .iter()
            .find(|v| v.version == installed.active)
            .map(|v| v.sha256.clone())
            .unwrap_or_default();
        let model_read = granted.contains(&permissions::Permission::ModelRead);
        CallSetup {
            plugin_id: manifest.id.clone(),
            version: installed.active.clone(),
            sha256,
            wasm_path: dir.join(&manifest.entry),
            package_dir: dir,
            data_dir: self.store.data_dir(&manifest.id),
            granted,
            document: input.document.clone().filter(|_| model_read),
            selection: input.selection.clone(),
            limits: self.limits,
        }
    }

    fn context(installed: &Installed, input: &CallInput, prefix: &str) -> String {
        serde_json::json!({
            "api": "0.1",
            "locale": input.locale,
            "revisionId": input.revision_id,
            "idPrefix": prefix,
            "settings": installed.settings,
        })
        .to_string()
    }

    fn journal_call(&self, id: &str, version: &str, what: &str, result: &Result<CallReport>) {
        let mut entry = match result {
            Ok(report) => {
                let mut entry = Entry::new(id, Some(version), "call", true, format!("{what} fuel={} {}", report.fuel_used, report.logs.join(" | ")));
                entry.duration_ms = Some(report.duration_ms);
                entry
            }
            Err(error) => Entry::new(id, Some(version), "call", false, format!("{what} {}: {}", error.code, error.message)),
        };
        entry.detail.truncate(2000);
        self.journal.append(&entry);
    }

    /// Run `feature.build` or `command.run` and parse the Command API plan.
    pub fn plan(&self, id: &str, kind: PlanKind, contribution_id: &str, params: &serde_json::Value, input: &CallInput) -> Result<Plan> {
        let (installed, manifest, granted) = self.loaded(id)?;
        let list = match kind {
            PlanKind::Feature => &manifest.contributes.features,
            PlanKind::Command => &manifest.contributes.commands,
        };
        let contribution = list
            .iter()
            .find(|c| c.id == contribution_id)
            .ok_or_else(|| PluginError::host("PLUGIN_CONTRIBUTION_MISSING", format!("{contribution_id} is not provided by this plugin")).for_plugin(id))?;
        let values = forms::resolve_values(&contribution.params, params)
            .map_err(|e| PluginError::host("PLUGIN_PARAMS_INVALID", e).for_plugin(id))?;
        let prefix = format!("p{}_", &uuid::Uuid::new_v4().simple().to_string()[..10]);
        let context = Self::context(&installed, input, &prefix);
        let params_json = serde_json::Value::Object(values).to_string();
        let interface = match kind {
            PlanKind::Feature => ("feature", "build"),
            PlanKind::Command => ("command", "run"),
        };
        let result = self.runtime().call(self.setup(&installed, &manifest, granted, input), |store, instance| {
            runtime::call_strings(store, instance, interface.0, interface.1, (contribution_id, &params_json, &context))
        });
        self.journal_call(id, &installed.active, contribution_id, &result.as_ref().map(|(_, r)| r.clone()).map_err(Clone::clone));
        let (output, report) = result?;
        let (message, commands) = match kind {
            PlanKind::Feature => (None, parse_commands(id, &output)?),
            PlanKind::Command => {
                #[derive(serde::Deserialize)]
                #[serde(deny_unknown_fields)]
                struct CommandPlan {
                    #[serde(default)]
                    message: Option<String>,
                    commands: serde_json::Value,
                }
                let plan: CommandPlan = serde_json::from_str(&output).map_err(|e| output_invalid(id, e.to_string()))?;
                let commands = if plan.commands.as_array().is_some_and(Vec::is_empty) {
                    vec![]
                } else {
                    parse_commands(id, &plan.commands.to_string())?
                };
                (plan.message.map(|m| m.chars().take(300).collect()), commands)
            }
        };
        if kind == PlanKind::Feature && commands.is_empty() {
            return Err(output_invalid(id, "A feature must return at least one command".into()));
        }
        check_prefix(id, &commands, &prefix)?;
        Ok(Plan {
            plugin_id: id.to_owned(),
            plugin_name: manifest.name.clone(),
            version: installed.active.clone(),
            title: contribution.title.clone(),
            message,
            commands,
            report,
        })
    }

    /// Run an importer on the bytes of the file the user picked.
    pub fn import(&self, id: &str, importer_id: &str, file_name: &str, data: &[u8], params: &serde_json::Value, input: &CallInput) -> Result<Plan> {
        let (installed, manifest, granted) = self.loaded(id)?;
        let importer = manifest
            .contributes
            .importers
            .iter()
            .find(|c| c.id == importer_id)
            .ok_or_else(|| PluginError::host("PLUGIN_CONTRIBUTION_MISSING", importer_id.to_owned()).for_plugin(id))?;
        let values = forms::resolve_values(&importer.params, params).map_err(|e| PluginError::host("PLUGIN_PARAMS_INVALID", e).for_plugin(id))?;
        let prefix = format!("p{}_", &uuid::Uuid::new_v4().simple().to_string()[..10]);
        let context = Self::context(&installed, input, &prefix);
        let params_json = serde_json::Value::Object(values).to_string();
        let result = self.runtime().call(self.setup(&installed, &manifest, granted, input), |store, instance| {
            runtime::call_import(store, instance, (importer_id, file_name, data, &params_json, &context))
        });
        self.journal_call(id, &installed.active, importer_id, &result.as_ref().map(|(_, r)| r.clone()).map_err(Clone::clone));
        let (output, report) = result?;
        let commands = parse_commands(id, &output)?;
        check_prefix(id, &commands, &prefix)?;
        Ok(Plan {
            plugin_id: id.to_owned(),
            plugin_name: manifest.name.clone(),
            version: installed.active.clone(),
            title: importer.title.clone(),
            message: None,
            commands,
            report,
        })
    }

    /// Run an exporter; the caller writes the bytes to the user-picked path.
    pub fn export(&self, id: &str, exporter_id: &str, params: &serde_json::Value, input: &CallInput) -> Result<(Vec<u8>, CallReport)> {
        let (installed, manifest, granted) = self.loaded(id)?;
        let exporter = manifest
            .contributes
            .exporters
            .iter()
            .find(|c| c.id == exporter_id)
            .ok_or_else(|| PluginError::host("PLUGIN_CONTRIBUTION_MISSING", exporter_id.to_owned()).for_plugin(id))?;
        let values = forms::resolve_values(&exporter.params, params).map_err(|e| PluginError::host("PLUGIN_PARAMS_INVALID", e).for_plugin(id))?;
        let document = input
            .document
            .clone()
            .ok_or_else(|| PluginError::host("PLUGIN_NEEDS_CAD_IR", "Export needs a CAD IR v2 model").for_plugin(id))?;
        let context = Self::context(&installed, input, "");
        let params_json = serde_json::Value::Object(values).to_string();
        let result = self.runtime().call(self.setup(&installed, &manifest, granted, input), |store, instance| {
            runtime::call_export(store, instance, (exporter_id, &document, &params_json, &context))
        });
        self.journal_call(id, &installed.active, exporter_id, &result.as_ref().map(|(_, r)| r.clone()).map_err(Clone::clone));
        result
    }

    /// Render a declarative panel. The view is validated before the UI sees it.
    pub fn panel(&self, id: &str, panel_id: &str, input: &CallInput) -> Result<(serde_json::Value, CallReport)> {
        let (installed, manifest, granted) = self.loaded(id)?;
        if !manifest.contributes.panels.iter().any(|p| p.id == panel_id) {
            return Err(PluginError::host("PLUGIN_CONTRIBUTION_MISSING", panel_id.to_owned()).for_plugin(id));
        }
        let context = Self::context(&installed, input, "");
        let result = self.runtime().call(self.setup(&installed, &manifest, granted, input), |store, instance| {
            runtime::call_panel(store, instance, (panel_id, &context))
        });
        self.journal_call(id, &installed.active, panel_id, &result.as_ref().map(|(_, r)| r.clone()).map_err(Clone::clone));
        let (output, report) = result?;
        let view = super::panel_view::parse(&output, &manifest).map_err(|e| output_invalid(id, e))?;
        Ok((view, report))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PlanKind {
    Feature,
    Command,
}

fn not_installed(id: &str) -> PluginError {
    PluginError::host("PLUGIN_NOT_INSTALLED", "The plugin is not installed").for_plugin(id)
}

fn output_invalid(id: &str, message: String) -> PluginError {
    PluginError::plugin("PLUGIN_OUTPUT_INVALID", message).for_plugin(id)
}

fn parse_commands(id: &str, output: &str) -> Result<Vec<Command>> {
    let commands: Vec<Command> = serde_json::from_str(output).map_err(|e| output_invalid(id, e.to_string()))?;
    if commands.len() > 256 {
        return Err(output_invalid(id, "More than 256 commands".into()));
    }
    Ok(commands)
}

/// New entities must use the per-call prefix so plugin output can never
/// collide with (or masquerade as) existing model IDs.
fn check_prefix(id: &str, commands: &[Command], prefix: &str) -> Result<()> {
    for command in commands {
        let new_id = match command {
            Command::AddFeature { feature } => Some(&feature.id),
            Command::AddBody { body } => Some(&body.id),
            Command::AddParameter { parameter } => Some(&parameter.id),
            _ => None,
        };
        if let Some(new_id) = new_id {
            if !new_id.starts_with(prefix) {
                return Err(output_invalid(id, format!("New id {new_id} must start with idPrefix")));
            }
        }
    }
    Ok(())
}
