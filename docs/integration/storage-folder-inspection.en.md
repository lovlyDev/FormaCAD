# Original-folder STEP inspection — exact patch

[Русский](storage-folder-inspection.md) · [Host API](../en/storage-folder-host-api.md)

Status: **applied after lovlyDev approved all 10 files** on 4 November 2026.
Strict compilation and native tests for the new revision are pending. Patch base:
`4e16dd42da581b664618837a2c0e1041acfc1e2c`, Issue #31 / PR #35.
Implementation owner: wenlixxq; scope agreement and acceptance: lovlyDev.

## Contract

Existing `inspect_model` retains managed `{projectId, revisionId}` calls. Folder
calls additionally require `{sourceId, expectedCommitId}` together. Tauri supplies
the invoking window; `WindowSources::capture_read` checks window, original
opening, project ID and commit and returns a bound `RepositoryRead`. Unsupported
platforms/builds without native client return the translated technical code
`PROJECT_FOLDER_UNSUPPORTED`, never managed fallback or a pretend `null` result.
Partial source references are refused.

Both backends use one `run_bound`: select the saved revision/descriptor from the
original capture, read bytes through the common reader, recheck after the CAD
queue and after the actual worker. The geometry algorithm and worker API remain
unchanged. Final byte/cancellation checks remain. The legacy inline-checksum
`PROJECT_SNAPSHOT_CHANGED` result is preserved by explicitly adapting the
existing integrity error. No writer grant or recovery is introduced.

Frontend `inspectSavedModel` accepts the existing optional `FolderSourceRef`;
previous calls remain compatible. Browser folder calls use the existing
unsupported-environment translation. No button or new user message is added.

## Ten exact paths

Rust paths are relative to `apps/desktop/src-tauri/src/`:

- `project_repository/window_sources/mod.rs`: register capture module/use common entry.
- `project_repository/window_sources/supported.rs`: move capture to the platform-stable module.
- `project_repository/window_sources/assets.rs`: use the same `capture_read`.
- `project_repository/window_sources/assets/tests.rs`: existing checks through public host capture.
- new `project_repository/window_sources/capture.rs`: common capture/refusal on every platform.
- `model_inspection.rs`: optional source parameters on existing IPC.
- `model_inspection/read.rs`: single managed/FCP1 algorithm, existing test seam retained.
- `model_inspection/read/tests.rs`: only `pub(super)` on two existing fixture helpers.
- new `model_inspection/read/folder_tests.rs`: actual worker and late folder close.
- `apps/desktop/src/lib/modelInspection.ts`: optional client source reference.

## Evidence and incomplete gates

Exact diff: [storage-folder-inspection.patch](storage-folder-inspection.patch).
`git apply --check` and staged Rust formatting passed. The base passed
[Storage comparison](https://github.com/lovlyDev/FormaCAD/actions/runs/41810364767);
the subsequently applied patch still needs its own compilation and native tests.

The new native regression builds a real 40×20×10 STEP, saves FCP1, and installs a
conflicting managed row sharing its ID with intentionally invalid STEP bytes.
Folder inspection must return 8000 mm³ and the correct bounds. After a subsequent
actual worker completes, closing the original folder must reject the late result.
Folder SQL and app state are compared with their original bytes/state. Existing
managed native tests are preserved. Test presence is not PASS; after application,
strict Clippy and real native tests on the exact SHA are required.

CAD gate/status/cancel conservatively still use project ID. Equal IDs can share
the queue; task identity must be separated by source before enabling folder UI.
This does not complete workspace/source state, other geometry consumers, model
writes/permissions, history IPC, recent index or all of stage 4. Spec §5 requires
agreement for model_inspection/frontend API. No native/Cargo/C++/worker protocol,
permissions, SQL schema or version changes.
