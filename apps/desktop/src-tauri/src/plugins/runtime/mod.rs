//! Wasmtime execution: one fresh component instance per call, with fuel,
//! a 256 MB memory cap and an epoch-based wall-clock timeout. A trap, a
//! timeout or a resource limit ends only that call and becomes a plugin error.
pub mod host;

use super::{
    error::{PluginError, Result},
    permissions::PermissionSet,
    wasm_check::export_name,
};
use host::HostState;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock},
    time::{Duration, Instant},
};
use wasmtime::{
    component::{Component, ComponentType, Instance, Lift, Linker, Lower},
    Config, Engine, Store, Trap,
};

pub const MEMORY_LIMIT: usize = 256 * 1024 * 1024;
pub const DEFAULT_FUEL: u64 = 5_000_000_000;
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
const TICK: Duration = Duration::from_millis(50);

/// `types.plugin-error` as seen by the host.
#[derive(Debug, Clone, ComponentType, Lift, Lower)]
#[component(record)]
pub struct WitError {
    code: String,
    message: String,
}

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub fuel: u64,
    pub timeout: Duration,
    pub memory: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            fuel: DEFAULT_FUEL,
            timeout: DEFAULT_TIMEOUT,
            memory: MEMORY_LIMIT,
        }
    }
}

/// What one call needs besides its arguments.
pub struct CallSetup {
    pub plugin_id: String,
    pub version: String,
    pub sha256: String,
    pub wasm_path: PathBuf,
    pub package_dir: PathBuf,
    pub data_dir: PathBuf,
    pub granted: PermissionSet,
    pub document: Option<String>,
    pub selection: Option<String>,
    pub limits: Limits,
}

/// Side results of a call that the UI shows or the journal records.
#[derive(Debug, Default, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CallReport {
    pub notices: Vec<host::Notice>,
    pub logs: Vec<String>,
    pub duration_ms: u64,
    pub fuel_used: u64,
}

pub struct Runtime {
    engine: Engine,
    cache: Mutex<HashMap<String, Component>>,
}

impl Runtime {
    /// Process-wide runtime with one epoch ticker thread.
    pub fn global() -> &'static Arc<Runtime> {
        static RUNTIME: OnceLock<Arc<Runtime>> = OnceLock::new();
        RUNTIME.get_or_init(|| {
            let runtime = Arc::new(Runtime::new().expect("Wasmtime engine configuration is valid"));
            let engine = runtime.engine.clone();
            std::thread::Builder::new()
                .name("plugin-epoch".into())
                .spawn(move || loop {
                    std::thread::sleep(TICK);
                    engine.increment_epoch();
                })
                .expect("plugin epoch thread starts");
            runtime
        })
    }

    fn new() -> wasmtime::Result<Self> {
        let mut config = Config::new();
        config
            .wasm_component_model(true)
            .consume_fuel(true)
            .epoch_interruption(true)
            .max_wasm_stack(2 * 1024 * 1024)
            // Linear memory beyond the limiter is never reserved up front.
            .memory_reservation(MEMORY_LIMIT as u64 + 64 * 1024 * 1024);
        Ok(Self {
            engine: Engine::new(&config)?,
            cache: Mutex::default(),
        })
    }

    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    fn component(&self, setup: &CallSetup) -> Result<Component> {
        let key = format!("{}@{}#{}", setup.plugin_id, setup.version, setup.sha256);
        if let Some(component) = self
            .cache
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get(&key)
        {
            return Ok(component.clone());
        }
        let bytes = std::fs::read(&setup.wasm_path)?;
        let component = Component::new(&self.engine, &bytes)
            .map_err(|error| PluginError::package("PLUGIN_WASM_INVALID", format!("{error:#}")))?;
        let mut cache = self.cache.lock().unwrap_or_else(|p| p.into_inner());
        cache.retain(|existing, _| !existing.starts_with(&format!("{}@", setup.plugin_id)));
        cache.insert(key, component.clone());
        Ok(component)
    }

    /// Run `body` against a fresh instance under the configured limits.
    pub fn call<R>(
        &self,
        setup: CallSetup,
        body: impl FnOnce(
            &mut Store<HostState>,
            &Instance,
        ) -> wasmtime::Result<std::result::Result<R, WitError>>,
    ) -> Result<(R, CallReport)> {
        let id = setup.plugin_id.clone();
        let component = self.component(&setup)?;
        let mut state = HostState::new(
            &setup.plugin_id,
            setup.granted.clone(),
            setup.package_dir,
            setup.data_dir,
            setup.limits.memory,
        );
        state.document = setup.document;
        if let Some(selection) = setup.selection {
            state.selection = selection;
        }
        state.network = tokio::runtime::Handle::try_current().ok();
        let mut linker = Linker::new(&self.engine);
        host::link(&mut linker, &setup.granted)
            .map_err(|e| PluginError::host("PLUGIN_HOST", format!("{e:#}")))?;
        let mut store = Store::new(&self.engine, state);
        store.limiter(|state| &mut state.limiter);
        store
            .set_fuel(setup.limits.fuel)
            .map_err(|e| PluginError::host("PLUGIN_HOST", e.to_string()))?;
        let ticks = (setup.limits.timeout.as_millis() / TICK.as_millis()).max(1) as u64;
        store.set_epoch_deadline(ticks);
        store.epoch_deadline_trap();
        let started = Instant::now();
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let instance = linker.instantiate(&mut store, &component)?;
            body(&mut store, &instance)
        }));
        let fuel_left = store.get_fuel().unwrap_or(0);
        let state = store.data();
        let report = CallReport {
            notices: state.notices.clone(),
            logs: state
                .logs
                .iter()
                .map(|(level, line)| format!("{level:?}: {line}"))
                .collect(),
            duration_ms: started.elapsed().as_millis() as u64,
            fuel_used: setup.limits.fuel.saturating_sub(fuel_left),
        };
        let memory_denied = state.limiter.memory_denied;
        match outcome {
            Err(_) => Err(PluginError::host(
                "PLUGIN_HOST_PANIC",
                "The plugin host failed during the call",
            )
            .for_plugin(&id)),
            Ok(Err(error)) => Err(classify(&error, memory_denied).for_plugin(&id)),
            Ok(Ok(Err(error))) => {
                let code = if error.code.is_empty() {
                    "PLUGIN_ERROR".into()
                } else {
                    error.code
                };
                Err(PluginError {
                    code,
                    message: error.message.chars().take(1000).collect(),
                    origin: super::error::Origin::Plugin,
                    plugin_id: Some(id),
                })
            }
            Ok(Ok(Ok(value))) => Ok((value, report)),
        }
    }
}

fn classify(error: &wasmtime::Error, memory_denied: bool) -> PluginError {
    let detail = format!("{error:#}");
    match error.downcast_ref::<Trap>() {
        Some(Trap::OutOfFuel) => PluginError::plugin("PLUGIN_FUEL_EXHAUSTED", detail),
        Some(Trap::Interrupt) => PluginError::plugin("PLUGIN_TIMEOUT", detail),
        Some(_) if memory_denied => PluginError::plugin("PLUGIN_MEMORY_LIMIT", detail),
        Some(_) => PluginError::plugin("PLUGIN_TRAP", detail),
        None if memory_denied => PluginError::plugin("PLUGIN_MEMORY_LIMIT", detail),
        None if detail.contains("import") || detail.contains("instantiat") => {
            PluginError::plugin("PLUGIN_INSTANTIATE", detail)
        }
        None => PluginError::plugin("PLUGIN_TRAP", detail),
    }
}

fn lookup(
    store: &mut Store<HostState>,
    instance: &Instance,
    interface: &str,
    func: &str,
) -> wasmtime::Result<wasmtime::component::ComponentExportIndex> {
    let parent = instance
        .get_export_index(&mut *store, None, &export_name(interface))
        .ok_or_else(|| {
            wasmtime::format_err!("component does not export {}", export_name(interface))
        })?;
    instance
        .get_export_index(&mut *store, Some(&parent), func)
        .ok_or_else(|| wasmtime::format_err!("{} has no function {func}", export_name(interface)))
}

/// `feature.build`, `command.run` and `panel.render` share a string ABI.
pub fn call_strings(
    store: &mut Store<HostState>,
    instance: &Instance,
    interface: &str,
    func: &str,
    args: (&str, &str, &str),
) -> wasmtime::Result<std::result::Result<String, WitError>> {
    let index = lookup(store, instance, interface, func)?;
    let typed = instance
        .get_typed_func::<(&str, &str, &str), (std::result::Result<String, WitError>,)>(
            &mut *store,
            &index,
        )?;
    Ok(typed.call(&mut *store, args)?.0)
}

pub fn call_panel(
    store: &mut Store<HostState>,
    instance: &Instance,
    args: (&str, &str),
) -> wasmtime::Result<std::result::Result<String, WitError>> {
    let index = lookup(store, instance, "panel", "render")?;
    let typed = instance.get_typed_func::<(&str, &str), (std::result::Result<String, WitError>,)>(
        &mut *store,
        &index,
    )?;
    Ok(typed.call(&mut *store, args)?.0)
}

pub fn call_import(
    store: &mut Store<HostState>,
    instance: &Instance,
    args: (&str, &str, &[u8], &str, &str),
) -> wasmtime::Result<std::result::Result<String, WitError>> {
    let index = lookup(store, instance, "importer", "import-file")?;
    let typed = instance.get_typed_func::<(&str, &str, &[u8], &str, &str), (std::result::Result<String, WitError>,)>(&mut *store, &index)?;
    Ok(typed.call(&mut *store, args)?.0)
}

pub fn call_export(
    store: &mut Store<HostState>,
    instance: &Instance,
    args: (&str, &str, &str, &str),
) -> wasmtime::Result<std::result::Result<Vec<u8>, WitError>> {
    let index = lookup(store, instance, "exporter", "export-file")?;
    let typed = instance
        .get_typed_func::<(&str, &str, &str, &str), (std::result::Result<Vec<u8>, WitError>,)>(
            &mut *store,
            &index,
        )?;
    Ok(typed.call(&mut *store, args)?.0)
}
