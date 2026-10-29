use super::*;

#[test]
fn default_profile_preserves_configured_paths() {
    let mut config = Config::default();
    let original = serde_json::to_value(&config).unwrap();
    configure(&mut config, None).unwrap();
    assert_eq!(serde_json::to_value(&config).unwrap(), original);
}

#[test]
fn explicit_profile_redirects_all_tauri_app_directories_without_creating_it() {
    let root = tempfile::tempdir().unwrap();
    let profile = root.path().join("profile with spaces").join("профиль");
    let mut config = Config::default();
    configure(&mut config, Some(profile.clone().into_os_string())).unwrap();
    assert_eq!(
        config.app.app_directories_override,
        Some(AppDirectoriesOverride::Root(profile.clone()))
    );
    assert!(!profile.exists());
    root.close().unwrap();
}

#[test]
fn invalid_override_leaves_config_and_filesystem_untouched() {
    let mut config = Config::default();
    let original = serde_json::to_value(&config).unwrap();
    for path in ["", "relative/profile", "./profile", "../profile"] {
        assert_eq!(
            configure(&mut config, Some(path.into())),
            Err("FORMA_DATA_DIR must be an absolute path")
        );
        assert_eq!(serde_json::to_value(&config).unwrap(), original);
    }
}

#[cfg(windows)]
#[test]
fn windows_drive_relative_and_root_relative_profiles_are_rejected() {
    for path in ["C:profile", "\\profile"] {
        assert!(configure(&mut Config::default(), Some(path.into())).is_err());
    }
}
