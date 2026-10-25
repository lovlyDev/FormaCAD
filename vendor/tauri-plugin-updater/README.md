# Forma updater dependency patch / Исправление зависимости updater

## Русский

Это локальная копия **официального `tauri-plugin-updater` 2.11.0**, а не новая реализация механизма обновления. Она сохраняет API, проверку подписи, правила установки и поддержку архивов всех платформ. Основное приложение подключает её явно через Cargo; номер версии плагина не изменён.

### Происхождение

- Репозиторий: [tauri-apps/plugins-workspace](https://github.com/tauri-apps/plugins-workspace).
- Исходный commit: `6aa2854f314481a459be1189b02c65a2450789ab`, каталог `plugins/updater`, по `.cargo_vcs_info.json` опубликованного crate.
- [Опубликованный crate 2.11.0](https://crates.io/crates/tauri-plugin-updater/2.11.0).
- SHA-256 архива crate: `b28d8cabdeb0564f03ae261963de4bc3d98321cd3d213e76a81b7d344e5df606`. Он совпадает с исходным registry checksum в Cargo.lock Forma и проверенным локальным архивом.
- Без изменения скопированы `src/`, `permissions/`, `build.rs`, `api-iife.js` и три файла лицензий. Использован нормализованный Cargo.toml из crate. Кэш Cargo, сторонний lockfile и история изменений не включены.

### Единственное изменение upstream-кода

В Windows-зависимости `zip` версия `4` заменена на `9.0.0`. `optional = true`, `default-features = false` и все существующие features updater сохранены. ZIP 9 сохраняет используемые здесь `ZipArchive::new` и `extract`; правки Rust-исходников не потребовались. Транспорт, проверка подписи до установки, выбор установщика и Linux/macOS TAR-маршруты не менялись.

ZIP 9 требует Rust 1.88: это фактический минимум для Windows с включённой ZIP-поддержкой, несмотря на исходное поле `rust-version` плагина. Forma использует Rust 1.97.1.

Причина — исправления ZIP [RUSTSEC-2026-0367](https://rustsec.org/advisories/RUSTSEC-2026-0367.html), [RUSTSEC-2026-0368](https://rustsec.org/advisories/RUSTSEC-2026-0368.html), [RUSTSEC-2026-0369](https://rustsec.org/advisories/RUSTSEC-2026-0369.html). Отключение feature `zip` не является заменой: оно также изменяет платформенные возможности updater. Никакие проверки безопасности или подписи не отключаются.

Лицензия upstream — Apache-2.0 **или** MIT; сохранены [Apache-2.0](LICENSE_APACHE-2.0), [MIT](LICENSE_MIT) и [SPDX](LICENSE.spdx). При следующем обновлении сверяйте официальный crate и удалите локальную копию только после появления совместимой исправленной зависимости upstream. Сборку, аудит и поведение всех платформ нужно проверить в контексте приложения; само наличие этой копии не подтверждает установку обновления.

## English

This is a local copy of **official `tauri-plugin-updater` 2.11.0**, retaining its API, signature verification, installation rules and platform archive support. The application selects it explicitly through Cargo; the plugin version remains unchanged.

Provenance: `tauri-apps/plugins-workspace`, commit `6aa2854f314481a459be1189b02c65a2450789ab`, path `plugins/updater`, as recorded by the published crate's `.cargo_vcs_info.json`. The crate archive SHA-256 is `b28d8cabdeb0564f03ae261963de4bc3d98321cd3d213e76a81b7d344e5df606`, matching Forma's original registry lock entry and the verified cached archive. Sources, permissions, build script, global API script and license files are copied unchanged; Cargo.toml is the registry-normalized manifest. Cargo cache markers, the upstream lockfile and changelog are excluded.

The **only upstream change** replaces the Windows optional ZIP dependency `4` with `9.0.0`, preserving `default-features = false` and every existing updater feature. ZIP 9 retains the `ZipArchive::new` and `extract` APIs used here, so no Rust source changes are required. Transport, pre-install signature checks, installer selection and Linux/macOS TAR behavior are unchanged.

ZIP 9 requires Rust 1.88, raising the effective Windows ZIP-enabled minimum above the plugin's original `rust-version` declaration. Forma uses Rust 1.97.1.

This addresses the three ZIP extraction advisories linked above without suppressing audit findings or disabling platform functionality. Upstream dual Apache-2.0/MIT licensing and all three license files are retained. Revisit this patch when upstream provides a compatible fixed dependency. Application-level audit, builds and platform behavior still require verification; this source copy alone is not evidence of a successfully installed update.
