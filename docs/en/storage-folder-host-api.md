# Independent FCP1 folder Rust API

[Documentation](index.md) · [Русский](../ru/storage-folder-host-api.md) · [Format](storage-formats.md)

`project_repository::sqlite_folder::api` exposes the backend to ordinary Windows
and Linux x86_64/aarch64 Rust builds. This revision still requires compilation
and runtime verification. It is a trusted host API; UI/IPC and app index are not
connected. Selecting the folder never imports it into AppData. LegacyManaged
and the V2 reader remain separate.

## Reading

The host explicitly selects a path, calls `FolderProject::open` and obtains an
opaque `ReadView` with `capture`. The view retains its physical opening and
consistent SQL/history state, is not serializable and provides no write authority.
Asset/history reads, `recheck` and `resolve_commit` use that original view.
Project/history/commit getters expose snapshot data; delayed results still need
rechecking before acceptance.

Common `ProjectRepository::independent_folder` delegates to the same backend
without app SQL or a managed row. `close` revokes just that repository handle:
another opening with the same ID remains valid, and old snapshots cannot enter
a new handle. Checks surround awaited operations. Readers never acquire writer
leases, repair journals or run recovery.

Current performance limitation: FCP1 capture still validates the complete bounded
history archive and commit chain. It is not a lazy metadata-only read; optimization
and measurements remain acceptance work. Managed capture retains its archive-free
read path. Sharing an API does not prove equal read cost across backends.

## Publication and CAD

After the explicit user decision, the host invokes `approve_write` for the
original view. Single-use `WriteApproval` retains the original saved-state stamp.
Project IDs, UI tokens and settings cannot reconstruct it. It never replaces
the actual OS writer lease independently acquired at publication.

`create` requires an empty destination and valid project/assets/history. `commit`
publishes a prepared backend candidate; `apply_history` uses the common undo/redo
planner. UI must never supply arbitrary Project data to these trusted Rust entry
points. `save_copy` needs no source writer approval, creates new identity and
preserves portable history.

`apply_model` takes the original view, `ModelIntent` and trusted `ModelRuntime`
(task map, CAD gate and machine paths). Project ID and expected revision come
from the original view. `FolderApplyHost` calls existing `repository_apply::apply`
and real `execution::build`, without a second geometry algorithm. No-op accepts
no approval and obtains no writer lease; an actual edit without approval is
refused. Runtime paths are never accepted from IPC.

## Outcomes and verification

`V2Backup::create` produces an exact independently reread backup of an explicitly
selected published V2 session. This is blocking file IO: a future UI host must
schedule it on a blocking worker. The proof object is not serializable;
`BackupRecord` only describes its location, manifest SHA, generation and project
ID and cannot restore migration authority. `V2Backup::migrate` delegates to the
existing checked converter with a separate empty FCP1 destination; the source
and backup are never write targets. Destination refusal still returns backup
provenance in `MigrationReport`. Reopening the same path cannot inherit an older
opening's proof. Two new API tests check exact manifest bytes, project/history
preservation and close/reopen refusal; current results are not yet established.

`Publication::Committed` exposes saved project and UUID, not a new capability.
`RejectedBefore` retains pre-commit refusal. `Indeterminate` retains the known
UUID and technical error: never replay automatically or pretend rollback.
`ModelChange::Published` alone is not confirmed publication. `resolve_commit`
uses the original opening; reopening requires a fresh view and cannot restore
an older grant.

API tests use real isolated SQLite/CAS for reopen without app index, foreign
opening/grant refusal, read-only copy under another writer lease and pure common
reading. Fixtures are synthetic and never replace real-worker roundtrips.
The public `apply_model` has an explicit ignored
`api::worker_tests::real_worker_host_api_roundtrip`: a real 40×20×10 box,
grant-free no-op, refusal of an unapproved edit, close and independent-process
reopen with worker verification of 8000 mm³ and bounds. The workflow requires one
executed PASS; current-revision success is not yet established.
With `native-worker-client`, the API also exposes `SelectedStepFile`, opaque
`StepRead`, single-use `StepDestinationApproval` and `transfer_to_folder` using
the same `ModelRuntime`. This delegates to existing [STEP transfer](storage-step-transfer.md)
without old app SQL or recovered parametric history. Hash/read and approval are
blocking file operations; a UI host must schedule them away from its UI thread.
A new synthetic regression covers foreign opening and destination data added
after approval. The real STEP roundtrip now calls this public API. Exact-head
native-client verification remains required.
Production UI/index, actual USB/NAS and a genuine 1.1
fixture remain incomplete gates. This API does not change versions or release.
