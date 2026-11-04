# Independent FCP1 folder Rust API

[Documentation](index.md) · [Русский](../ru/storage-folder-host-api.md) · [Format](storage-formats.md)

`project_repository::sqlite_folder::api` exposes the backend to ordinary Windows
and Linux x86_64/aarch64 Rust builds. This revision still requires compilation
and runtime verification. It is a trusted host API; UI/IPC and app index are not
connected. Selecting the folder never imports it into AppData. LegacyManaged
and the V2 reader remain separate.

## Reading

The host explicitly selects a path and calls `FolderProject::open` for an opaque
`ReadView`. `capture_metadata` checks Project, cursor and declared commit seals
without reading event rows; `capture` additionally exports and validates the full
archive. The view retains its physical opening and original saved state, is not
serializable and provides no write authority.
Asset/history reads, `recheck` and `resolve_commit` use that original view.
Project/history/commit getters expose snapshot data; delayed results still need
rechecking before acceptance.

Common `ProjectRepository::independent_folder` delegates to the same backend
without app SQL or a managed row. `close` revokes just that repository handle:
another opening with the same ID remains valid, and old snapshots cannot enter
a new handle. Checks surround awaited operations. Readers never acquire writer
leases, repair journals or run recovery.

Common `capture` uses metadata mode; `capture_with_history` uses complete mode.
`read_history` performs complete validation under the original opening and compares
commit/project/cursor/archive seals against the original view, never legitimizing
a fresh state. `approve_write` also requires complete validation before granting.
Metadata confirms a declared archive hash, not archive bytes: event corruption
may leave metadata readable while history, copy and write are refused. CAS reads
from metadata views verify their own bytes without exporting the archive.
Project revision metadata and the commit chain are still read; this is not O(1)
or proof of equal backend costs. Current-revision measurements remain required.

Ignored `api::benchmark::measure_scoped_reads` times actual FCP1
`capture + recheck + drop` for metadata and complete modes: 12 alternating-order
samples after warmup, median/p95 for each of three sizes. It reuses synthetic
revision/chat/STEP-labelled fixtures from the earlier comparison, not CAD.
The temporary setup SQL database is removed before opening FCP1; the FCP1 file
is hash-compared before/after reads. `FCP1_READ_BENCHMARK` reports Project/archive/
CAS sizes, first API open and raw distributions. This is a debug/test profile
with unflushed filesystem caches; process RSS/time also include setup. It does
not prove release-app, cold-disk or USB/NAS performance, nor equate metadata and
complete validation guarantees. The workflow requires one executed test;
numerical results below apply only to their named SHA.

The measurement executed on `d6af326` in
[run 41739896245](https://github.com/lovlyDev/FormaCAD/actions/runs/41739896245),
job `126278491804`: 1 passed, 0 failed. Linux x86_64, debug/test, warm cache;
12 samples per mode. Times include capture, recheck and view drop, excluding
fixture preparation and first API open.

| Revisions | Project / archive bytes | Metadata median / p95, ms | Complete median / p95, ms |
| --- | --- | --- | --- |
| 4 | 3893 / 1103 | 8.550 / 8.773 | 9.987 / 10.265 |
| 32 | 33535 / 8161 | 18.428 / 19.753 | 23.283 / 25.000 |
| 128 | 133911 / 32355 | 50.621 / 54.103 | 65.600 / 75.710 |

The measurement asserted independence from removed setup SQL and unchanged FCP1
bytes. The same run passed strict Clippy with and without `native-worker-client`,
92 ordinary folder
backend tests (6 ignored) and the earlier storage-representation comparison.
These timings cover different validation work, never permission to weaken
complete write validation or proof of native CAD/installed-app acceptance.

## Publication and CAD

### Window opening registry

`FolderSources` is a nonpersistent Rust host registry, one instance per window.
`open` accepts an already explicitly selected host folder and returns
`OpenedFolder` with its original `ReadView` and opaque `FolderSourceId`. `lookup`
returns an Arc to that same opening, never reopens a path or keys by project ID.
Two folders sharing a project ID remain distinct. `close` immediately revokes
the original session. `close_all` permanently closes the window scope, including
pending openings; another window creates another registry.

The common `ProjectRepository::from_folder_source` reader accepts the actual
Arc returned by this registry. It does not reopen the path or create a managed
project. Each request wrapper binds its own snapshots; closing that wrapper
revokes only its own views. The window registry still owns the source lifetime:
closing its key revokes every wrapper, and reopening cannot revive old views.
The standalone `independent_folder` constructor retains ownership of its own
opening and closes it with its handle. A real-folder regression exercises both
wrapper isolation and source-wide revocation through the common history route.

Ready and pending sources together are limited to 16. Capacity is reserved before
file IO. The map mutex never crosses SQL/disk/worker/await operations. Cancellation
releases its slot; after map publication but before returning the result, the
guard also removes and revokes only its own opening. Physical SQL connection
close happens outside the map lock and may complete later. After a successful
response the host owns cleanup: close unused late results and all sources on
window shutdown.

Keys include the window scope, nonwrapping monotonic sequence and a random nonce
for each opening. Knowing another sequence number cannot create a valid key.
Old keys are never reused after close/reopen; foreign scopes and forged nonces
are refused. Keys are not persisted in index/settings and confer no writer grant.
`WriteApproval` still requires an independent host decision and complete source
recheck. Internal `PROJECT_SOURCE_LIMIT` currently belongs to the Rust-only API;
agreed UI/IPC wiring must add RU/EN translation at the display boundary. Tests use
actual folders and production RAII reservations for duplicate project IDs,
foreign nonce/window, close/reopen/drop, pre-IO quota and post-publish cancellation.

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
A dedicated SQL authorizer denies reads of `project_history_events`: metadata
must succeed while complete capture is refused. Fault regressions check refusal
of history/copy/write with a damaged archive and retention of the original view
after an external commit; an artificial "archive unread" flag is not the proof.
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
