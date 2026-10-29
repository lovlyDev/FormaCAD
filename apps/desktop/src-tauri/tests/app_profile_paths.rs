#![cfg(not(any(target_os = "android", target_os = "ios")))]
//! Exercise the configured Tauri resolver and enabled persistence plugin without
//! creating a real WebView or reading the user's ordinary application profile.

use forma_core::{app_profile, storage, updates};
use std::{collections::BTreeMap, fs, path::Path};
use tauri::{
    path::BaseDirectory,
    test::{mock_builder, mock_context, noop_assets, MockRuntime},
    App, Manager,
};
use tauri_plugin_window_state::{AppHandleExt, StateFlags, DEFAULT_FILENAME};

fn isolated_app(directory: &Path) -> App<MockRuntime> {
    let mut context = mock_context(noop_assets());
    context.config_mut().identifier = "studio.forma.cad".into();
    app_profile::configure(
        context.config_mut(),
        Some(directory.as_os_str().to_os_string()),
    )
    .unwrap();
    assert!(context.config().app.windows.is_empty());
    // Configure before build: the real plugin's setup immediately reads state.
    mock_builder()
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .build(context)
        .unwrap()
}

async fn close_owned_database(pool: sqlx::SqlitePool) {
    // SQLx returns dropped leases asynchronously. Own every slot before closing
    // the SQLite workers so no late return can keep a fixture file open.
    let mut connections = Vec::new();
    for _ in 0..pool.options().get_max_connections() {
        connections.push(pool.acquire().await.unwrap());
    }
    for connection in connections {
        connection.close().await.unwrap();
    }
    pool.close().await;
    assert_eq!(
        pool.size(),
        0,
        "Fixture database connections must be closed"
    );
}

#[tokio::test]
async fn actual_app_resolver_and_database_use_the_configured_profile() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    let app = isolated_app(&root);
    let resolver = app.path();
    for (actual, expected, base) in [
        (
            resolver.app_config_dir().unwrap(),
            root.clone(),
            BaseDirectory::AppConfig,
        ),
        (
            resolver.app_data_dir().unwrap(),
            root.clone(),
            BaseDirectory::AppData,
        ),
        (
            resolver.app_local_data_dir().unwrap(),
            root.clone(),
            BaseDirectory::AppLocalData,
        ),
        (
            resolver.app_cache_dir().unwrap(),
            root.join("caches"),
            BaseDirectory::AppCache,
        ),
        (
            resolver.app_log_dir().unwrap(),
            root.join("logs"),
            BaseDirectory::AppLog,
        ),
    ] {
        assert_eq!(actual, expected);
        assert_eq!(resolver.resolve("", base).unwrap(), expected);
        assert_eq!(
            resolver.resolve("fixture", base).unwrap(),
            expected.join("fixture")
        );
        assert!(actual.starts_with(&root));
    }
    // On Windows/Linux Tauri selects app_local_data_dir as its default WebView
    // profile. This checks that input path, not a claim of browser execution.
    assert_eq!(resolver.app_local_data_dir().unwrap(), root);
    let database = resolver.app_data_dir().unwrap().join("forma.sqlite");
    let pool = storage::open(&database).await.unwrap();
    let attached: Vec<(i64, String, String)> = sqlx::query_as("PRAGMA database_list")
        .fetch_all(&pool)
        .await
        .unwrap();
    let actual_database = attached.iter().find(|(_, name, _)| name == "main").unwrap();
    assert_eq!(
        Path::new(&actual_database.2).canonicalize().unwrap(),
        database.canonicalize().unwrap()
    );
    let tables: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM sqlite_master WHERE type='table' AND name IN ('projects','settings','agent_sessions')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(tables, 3);
    let migrations: i64 =
        sqlx::query_scalar("SELECT count(*) FROM _sqlx_migrations WHERE success=1")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(migrations > 0);
    let preferences = updates::Preferences(BTreeMap::from([
        ("forma.locale".into(), "en".into()),
        ("forma.theme".into(), "light".into()),
    ]));
    updates::save_preferences(&pool, &preferences)
        .await
        .unwrap();
    close_owned_database(pool).await;
    let reopened = storage::open(&database).await.unwrap();
    let stored: String =
        sqlx::query_scalar("SELECT value FROM settings WHERE key='ui_preferences'")
            .fetch_one(&reopened)
            .await
            .unwrap();
    let stored: BTreeMap<String, String> = serde_json::from_str(&stored).unwrap();
    assert_eq!(stored, preferences.0);
    close_owned_database(reopened).await;
    drop(app);
    temporary.close().unwrap();
}

#[test]
fn enabled_window_state_plugin_reads_and_saves_each_profile_independently() {
    let temporary = tempfile::tempdir().unwrap();
    let mut profiles = Vec::new();
    for (name, width, height, x, y) in [("first", 1290, 785, 41, 52), ("second", 910, 615, 93, 104)]
    {
        let root = temporary.path().join(name);
        fs::create_dir(&root).unwrap();
        let root = root.canonicalize().unwrap();
        let original = serde_json::json!({"main": {
            "width": width, "height": height, "x": x, "y": y,
            "prev_x": x, "prev_y": y, "maximized": false,
            "visible": true, "decorated": true, "fullscreen": false
        }});
        fs::write(
            root.join(DEFAULT_FILENAME),
            serde_json::to_vec(&original).unwrap(),
        )
        .unwrap();
        profiles.push((root, original));
    }
    for (root, original) in &profiles {
        let app = isolated_app(root);
        assert_eq!(app.handle().filename(), DEFAULT_FILENAME);
        // No mocked window overwrites the cached geometry: the preserved entry
        // proves plugin setup loaded this profile, and its real save route works.
        app.handle().save_window_state(StateFlags::all()).unwrap();
        let saved: serde_json::Value =
            serde_json::from_slice(&fs::read(root.join(DEFAULT_FILENAME)).unwrap()).unwrap();
        assert_eq!(&saved, original);
        drop(app);
    }
    for (root, original) in profiles {
        let saved: serde_json::Value =
            serde_json::from_slice(&fs::read(root.join(DEFAULT_FILENAME)).unwrap()).unwrap();
        assert_eq!(saved, original);
        assert_eq!(fs::read_dir(root).unwrap().count(), 1);
    }
    temporary.close().unwrap();
}
