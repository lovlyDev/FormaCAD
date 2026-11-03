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
Production UI/index, STEP/migration facades, actual USB/NAS and a genuine 1.1
fixture remain incomplete gates. This API does not change versions or release.
