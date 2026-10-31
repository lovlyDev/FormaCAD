pub mod account;
pub mod agents;
pub mod app_profile;
pub mod artifacts;
pub mod cad;
pub mod cad_document;
pub mod cad_ir;
pub mod cad_tasks;
pub mod cadpack;
pub mod conversion;
pub mod core;
mod export_destination;
pub mod files;
mod job;
pub mod model_face_section;
pub mod model_inspection;
pub mod model_measurement;
pub mod model_pair_measurement;
pub mod model_preview;
pub mod model_section;
pub mod modeling;
pub mod models;
#[cfg(feature = "native-worker-client")]
pub mod native;
pub mod permissions;
pub mod plugins;
pub mod processes;
pub mod project_access;
pub mod project_history;
pub mod project_repository;
pub mod project_storage_v2;
pub mod projects;
pub mod security;
pub mod sketch_analysis;
pub mod storage;
pub mod updates;
#[cfg(feature = "native-occt")]
pub mod worker_protocol;
use std::{collections::HashMap, sync::atomic::Ordering};
use tauri::{Emitter, Manager};
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut context = tauri::generate_context!();
    if let Err(error) =
        app_profile::configure(context.config_mut(), std::env::var_os("FORMA_DATA_DIR"))
    {
        eprintln!("Forma could not start: {error}");
        std::process::exit(1);
    }
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_deep_link::init())
        .manage(account::AccountState::default())
        .setup(|app| {
            let data = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data)?;
            let root = data.join("projects");
            std::fs::create_dir_all(&root)?;
            let logs = data.join("logs");
            std::fs::create_dir_all(&logs)?;
            let appender = tracing_appender::rolling::Builder::new()
                .rotation(tracing_appender::rolling::Rotation::DAILY)
                .filename_prefix("forma")
                .max_log_files(14)
                .build(logs)?;
            let (writer, guard) = tracing_appender::non_blocking(appender);
            let _ = tracing_subscriber::fmt()
                .with_writer(writer)
                .with_ansi(false)
                .with_env_filter("forma_core=info,warn")
                .try_init();
            app.manage(plugins::PluginState::new(&data));
            register_plugin_links(app)?;
            let pool = tauri::async_runtime::block_on(storage::open(&data.join("forma.sqlite")))?;
            app.manage(core::AppState {
                cad_tasks: Default::default(),
                project_access: Default::default(),
                root,
                pool,
                grants: tokio::sync::Mutex::new(HashMap::new()),
                tasks: tokio::sync::Mutex::new(HashMap::new()),
                writes: tokio::sync::Mutex::new(()),
                _log_guard: guard,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            projects::list_projects,
            projects::read_project_snapshot,
            projects::save_project_thumbnail,
            projects::delete_project,
            projects::interrupted_sessions,
            projects::acknowledge_recovery,
            projects::save_project,
            project_history::history_status,
            project_history::undo_model,
            project_history::redo_model,
            project_storage_v2::ipc::inspect_project_folder,
            project_storage_v2::ipc::import_project_folder,
            project_access::ipc::acquire_project_access,
            project_access::ipc::release_project_access,
            project_access::ipc::project_access_status,
            project_access::ipc::copy_project_for_editing,
            cad_tasks::ipc::cad_task_status,
            cad_tasks::ipc::cancel_cad_task,
            model_section::section_model,
            model_measurement::measure_model_reference,
            model_face_section::section_model_reference,
            model_pair_measurement::measure_model_pair,
            cadpack::export::export_project_bundle,
            cadpack::import_project_bundle,
            permissions::request_permission,
            permissions::permission_audit,
            permissions::get_confirmation_settings,
            permissions::set_confirmation_settings,
            permissions::resolve_permission,
            agents::health::detect_environment,
            agents::health::choose_cad_python,
            agents::custom::get_custom_agent_config,
            agents::custom::set_custom_agent_config,
            agents::session::plan_model,
            agents::session::cancel_task,
            cad::export_step,
            files::export_mesh,
            artifacts::read_project_file,
            modeling::apply_program,
            modeling::apply_ir_commands,
            sketch_analysis::analyze_sketch,
            model_preview::preview_model,
            model_preview::cancel_model_preview,
            cad_ir::history::compare_ir_revisions,
            model_inspection::inspect_model,
            conversion::convert_step,
            updates::update_support,
            updates::get_ui_preferences,
            updates::set_ui_preferences,
            updates::prepare_update,
            account::commands::account_status,
            account::commands::account_sign_in,
            account::commands::account_cancel_sign_in,
            account::commands::account_refresh,
            account::commands::account_sign_out,
            plugins::ipc::plugins_list,
            plugins::ipc::plugins_install_file,
            plugins::ipc::plugins_install_link,
            plugins::ipc::plugins_take_pending_link,
            plugins::ipc::plugins_install_confirm,
            plugins::ipc::plugins_install_cancel,
            plugins::ipc::plugins_set_enabled,
            plugins::ipc::plugins_set_active,
            plugins::ipc::plugins_set_settings,
            plugins::ipc::plugins_uninstall,
            plugins::ipc::plugins_journal,
            plugins::ipc::plugins_run,
            plugins::ipc::plugins_import,
            plugins::ipc::plugins_export,
            plugins::ipc::plugins_panel,
            plugins::capture::ipc::plugins_capture_pick,
            plugins::capture::ipc::plugins_capture_write,
            plugins::capture::ipc::plugins_capture_finish
        ])
        .build(context);
    match app {
        Ok(app) => app.run(|handle, event| {
            if matches!(event, tauri::RunEvent::ExitRequested { .. }) {
                let state = handle.state::<core::AppState>();
                tauri::async_runtime::block_on(async {
                    for task in state.tasks.lock().await.values() {
                        task.store(true, Ordering::Relaxed);
                    }
                });
            }
        }),
        Err(e) => {
            eprintln!("Forma could not start: {e}");
        }
    }
}

/// `formacad://plugins/install?...` links only queue a request for the UI,
/// which asks the user before anything is downloaded or installed.
fn register_plugin_links(
    app: &mut tauri::App,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    use tauri_plugin_deep_link::DeepLinkExt;
    // Installers register the scheme; development builds register it per user.
    #[cfg(all(debug_assertions, any(windows, target_os = "linux")))]
    let _ = app.deep_link().register_all();
    let state = app.state::<plugins::PluginState>();
    if let Ok(Some(urls)) = app.deep_link().get_current() {
        for url in urls {
            state.offer_link(url.as_str());
        }
    }
    let handle = app.handle().clone();
    app.deep_link().on_open_url(move |event| {
        for url in event.urls() {
            if handle
                .state::<plugins::PluginState>()
                .offer_link(url.as_str())
            {
                let _ = handle.emit("plugins://install-link", url.as_str());
            }
        }
    });
    Ok(())
}
