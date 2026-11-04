# Window-owned independent folder reading — exact patch

[Русский](storage-folder-window.md) · [Host API](../en/storage-folder-host-api.md)

Status: prepared, **not applied**; shared IPC scope needs owner agreement under
spec §5. Base: `61e9b0006b02eb239966f59a02a86065a9e0c619`.
Owner: wenlixxq; acceptance: lovlyDev; Issue #31 / PR #35.

## Contract

The patch binds the existing FCP1 reader to window lifetime. It adds no button,
model write, migration, or completed user-flow claim. Subsequent UI wiring must
translate technical errors into RU/EN at the display boundary and propagate the
source key through related modeling operations.

`open_project_folder()` receives its window from Tauri and acquires its scope
**before** awaiting the native folder picker. It opens only the explicitly
selected folder and returns `null` on cancellation or `{sourceId, commitId,
project}`. The key is neither a writer grant nor a persisted index entry. No UI
path argument, AppData project copy, or managed SQLite project row is created.

The existing `read_project_snapshot` retains managed `{id}` calls. Folder calls
use `{id, sourceId, expectedCommitId}`; the additional fields must occur together.
The original window/opening, project ID, commit and source view after response
copy are checked. Invalid keys cannot fall back to managed storage. No duplicate
snapshot-read IPC is introduced.

`close_project_source({sourceId})` closes only a source in the invoking window.
`WindowEvent::Destroyed` revokes the whole scope. Late picker responses retain
the original closed scope and fail. A new window reusing a label receives a new
scope; old keys stay invalid. Limits: 16 window scopes, each with at most 16 ready
and pending sources. No map mutex crosses picker, await, SQLite or disk IO.
Dropping host state also revokes every scope.

Unsupported FCP1 platforms compile the commands but refuse open/read with
`PROJECT_FOLDER_UNSUPPORTED`; this does not add macOS format support. No Cargo,
native, protocol, permissions, SQL schema, or version changes.

## Eight exact paths

- `apps/desktop/src-tauri/src/lib.rs`: managed state, Destroyed hook, two commands.
- `apps/desktop/src-tauri/src/projects.rs`: optional existing IPC parameters.
- `apps/desktop/src-tauri/src/project_repository/mod.rs`: module registration.
- `apps/desktop/src-tauri/src/project_repository/window_sources/mod.rs`: lifecycle/read facade.
- `apps/desktop/src-tauri/src/project_repository/window_sources/supported.rs`: scope map.
- `apps/desktop/src-tauri/src/project_repository/window_sources/ipc.rs`: native picker/close.
- `apps/desktop/src-tauri/src/project_repository/window_sources/supported/tests.rs`: real folder, foreign window, expected commit/ID, teardown, reused label and drop.
- `.github/workflows/storage-benchmark.yml`: explicit new regression with one-PASS guard.

## Evidence and limitations

Exact diff: [storage-folder-window.patch](storage-folder-window.patch).
`git apply --check` and rustfmt of staged Rust files passed. The patch is not
applied or compiled; test presence is not PASS evidence. After approval, run
strict Clippy, the new regression, existing folder packet and exact-head Quality.
The native picker and Destroyed hook need real desktop verification. UI,
model/geometry routing, recent index and later stages remain incomplete.
