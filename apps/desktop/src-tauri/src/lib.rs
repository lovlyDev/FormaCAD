pub mod agents;
pub mod artifacts;
pub mod cad;
pub mod cad_document;
pub mod conversion;
pub mod core;
pub mod files;
mod job;
pub mod modeling;
pub mod models;
pub mod permissions;
pub mod processes;
pub mod projects;
pub mod security;
pub mod storage;
pub mod updates;
use std::{collections::HashMap, sync::atomic::Ordering};
use tauri::Manager;
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .setup(|app| {
            let data = std::env::var_os("FORMA_DATA_DIR")
                .map(std::path::PathBuf::from)
                .unwrap_or(app.path().app_data_dir()?);
            if !data.is_absolute() {
                return Err("FORMA_DATA_DIR must be an absolute path".into());
            }
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
            let pool = tauri::async_runtime::block_on(storage::open(&data.join("forma.sqlite")))?;
            app.manage(core::AppState {
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
            projects::save_project_thumbnail,
            projects::delete_project,
            projects::interrupted_sessions,
            projects::acknowledge_recovery,
            projects::save_project,
            permissions::request_permission,
            permissions::permission_audit,
            permissions::get_confirmation_settings,
            permissions::set_confirmation_settings,
            permissions::resolve_permission,
            agents::detect_environment,
            agents::choose_cad_python,
            agents::plan_model,
            agents::cancel_task,
            cad::export_step,
            files::export_mesh,
            artifacts::read_project_file,
            modeling::apply_program,
            conversion::convert_step,
            updates::update_support,
            updates::get_ui_preferences,
            updates::set_ui_preferences,
            updates::prepare_update
        ])
        .build(tauri::generate_context!());
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
