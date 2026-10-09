# Settings, localization, and updates

[Documentation](index.md) · [Русский](../ru/settings-and-updates.md) · [Data](data.md) · [Releases](releases.md)

Settings is divided into AI agents, CAD environment, permissions, appearance, and privacy. The environment list reports detected tools and has a refresh action. The AI provider is selected per project; Custom CLI and confirmation settings remain local. Python/CadQuery selection is for older documents, while the bundled native kernel builds CAD IR v2. The interpreter button opens a system picker and checks the environment. See [AI and permissions](ai.md) for providers and the audit log, and [SettingsDialog.tsx](../../apps/desktop/src/app/dialogs/SettingsDialog.tsx) for implementation.

## Language and theme

Russian is the default; English is selected under Appearance. The choice changes the UI immediately and persists across launches. `ru.json` and `en.json` carry matching user-facing keys; backend errors are translated at display, while user text, code, and filenames remain original. Numbers, dates, and plural forms use the chosen locale. See [i18n](../../apps/desktop/src/i18n/index.ts) and [catalogs](../../apps/desktop/src/i18n/).

Light and dark themes change surfaces, text, borders, and the WebGL scene without resetting the model. The selected theme persists. Purple is the accent in both. If the webview loses its storage, the desktop app restores missing saved preferences from SQLite. This does not replay every incomplete action: pending AI tasks and one-time approvals do not resume after restart. See [theme.ts](../../apps/desktop/src/lib/theme.ts), [persistence.ts](../../apps/desktop/src/lib/persistence.ts), and [data](data.md).

## Checking and installing a version

The desktop app checks for updates shortly after startup and every six hours; the update button can check manually. A newer published version opens a dialog with its number, Markdown release notes, and Install / Later actions. Release-note links open in the system browser; unknown URL schemes are discarded. While an editor or operation is active, the prompt waits. Later closes the dialog; the available update stays visible on the button. No network or no release does not block startup. See [Updates.tsx](../../apps/desktop/src/components/Updates.tsx).

During download, a progress bar shows a percentage if the server supplies total size; otherwise it shows indeterminate progress. Before installation, Forma flushes preferences, checks for active operations, and snapshots SQLite via `VACUUM INTO`, including committed WAL entries. The Tauri updater verifies the package signature and installs it. The application identifier and data directory stay unchanged. Normal updates are intended to preserve projects, revisions, camera position, body visibility, drafts, theme, and language. See [update backend](../../apps/desktop/src-tauri/src/updates.rs) and [backup guidance](data.md).

Windows uses its installer; macOS and Linux AppImage have an in-app update path. For Linux DEB/RPM, install the newer package through the package manager instead; app data remains in its directory. GitHub update checks see only **published** releases: local 1.2.6 is not offered to 1.2.5 users. Platform signing on macOS/Windows and updater signing are separate; packaging conditions are in [releases](releases.md), verification in [status](status.md).
