# Data and recovery
[Documentation](index.md) · [Русский](../ru/data.md)

Forma keeps the identifier studio.forma.cad across releases. Installers replace program files, not the data directory.

| System | Default data directory |
| --- | --- |
| Windows | %APPDATA%/studio.forma.cad |
| macOS | ~/Library/Application Support/studio.forma.cad |
| Linux | $XDG_DATA_HOME/studio.forma.cad, normally ~/.local/share/studio.forma.cad |

FORMA_DATA_DIR can select an absolute isolated directory for testing.

- forma.sqlite contains projects, revisions, permissions, recovery events and interface preferences.
- projects/ stores model artifacts, attachments and recovery JSON.
- cad-python.txt stores the chosen interpreter path.
- WebView storage keeps UI state; matching preferences are also stored in SQLite and restore missing WebView values on startup.
- The window-state plugin preserves window position, size and maximized state.
- backups/ contains uniquely named SQLite snapshots created before in-app updates. No automatic backup deletion is performed.

Workspace state includes the last project, camera position/orientation/target/zoom, projection, rendering mode, grid, body visibility/selection, panel dimensions, tree folders, comparison, editor/chat drafts and motion state. Body visibility is keyed by model name and traversal position, not ephemeral Three.js UUIDs. A changed model topology can invalidate old selections; this is not persistent topological naming.

Pending AI operations, one-time approvals and open confirmation dialogs are not replayed after restarting. Completed data remains saved; interrupted jobs are marked for recovery. SQLite backups contain metadata and settings; model files remain in projects/. For a complete external backup, copy the entire data directory with Forma closed.

## Recovery
Close Forma, copy the entire data directory somewhere safe, then inspect backups/. To restore a pre-update database, preserve the current forma.sqlite and any -wal/-shm files separately and replace it with a selected backup while the app is closed. Keep projects/ alongside it. Never restore by copying over an open database.

Do not uninstall with a “delete application data” option when updating. Use the normal installer over the existing installation. Moving to another computer requires copying data separately and choosing a valid local Python path.
