# Independent FCP1 folder Rust API

[Documentation](index.md) · [Русский](../ru/storage-folder-host-api.md) · [Format](storage-formats.md)

`project_repository::sqlite_folder::api` exposes the backend to ordinary Windows
and Linux x86_64/aarch64 Rust builds. This revision still requires compilation
and runtime verification. It is a trusted host API; window selection/reading IPC
is connected below, while product UI and app index remain incomplete.
Selecting the folder never imports it into AppData. LegacyManaged
and the V2 reader remain separate.

## Reading

### Metadata refresh through the original source

`RepositoryRead::recheck_source` validates its original reader family, opening
and physical location without asserting that the saved project is unchanged.
It is not a substitute for `recheck` or geometry input validation.
`recapture_metadata` obtains a separate current metadata view through the same
retained repository, with original-source checks before and after. It does not
look up a path/window key again, mutate the old view or revive a closed opening.
Ordinary asset/history/IPC reads still require the full saved-state check.

This supports geometry consumers that explicitly compare their original revision,
document/source/import/preview seals before accepting unrelated metadata saves.
Those consumers are not connected to this refresh method yet. A new metadata
view does not inherit a LegacyManaged archive capture; history requires explicit
capture/read according to its backend contract. Request close revokes its whole
derived-reader family, while independently captured window requests remain live.

Three actual SQL/filesystem regressions cover LegacyManaged, V2 and FCP1 metadata
publication, old-view staleness, source/epoch retention and close/reopen refusal.
They also check pure read state and metadata/archive separation. New exact-head
Rust tests are pending; fixture presence is not a successful test result.

### Exact saved STEP properties

The existing `inspect_model` now accepts the same optional source/expected-commit
pair as metadata and attachment IPC. `inspectSavedModel` accepts a
`FolderSourceRef`. Managed calls keep their existing arguments. Invalid/partial
folder references fail before inspection; unsupported builds and browser folder
calls use the existing localized unsupported-environment error.

Both backends use one inspection algorithm over `RepositoryRead`. It selects the
saved STEP revision/descriptor, verifies bytes through the common reader, checks
the original view after waiting for the CAD gate and after the actual worker,
and retains byte/cancellation checks. The output property fields and geometry
algorithm are unchanged. There is no recovery or writer grant in this route.

The new native regression creates genuine 40×20×10 STEP in FCP1, then installs
invalid STEP bytes in a managed row with the same project ID. Folder inspection
must still report 8000 mm³ and correct bounds. Closing the original folder after
the real worker must refuse its late properties, while source SQL/app state stay
unchanged. This applied revision is not yet verified by native CI.

For FCP1 inspection, the task group now comes from the actual opening's existing
UUID, matching FolderApplyHost. `open_project_folder` returns it as `taskScopeId`
in addition to sourceId/commitId/project; FolderSourceRef includes this field for
workspace status filtering. It is not persisted or accepted as authority by
read/write/inspection IPC. The server derives the group from the bound view.
Managed/V2 retain their project-ID groups. Distinct folder openings with equal
project IDs have different groups; commit retains the group, reopen changes it.
The global one-worker semaphore and individual task UUID/cancel API are unchanged.

Group separation/stability and actual registered inspection group during the
real worker passed on `d242babdb83b75c49cdcbfcfd4e18b80776c07d3`:
[comparison](https://github.com/lovlyDev/FormaCAD/actions/runs/41813913525) and
[native](https://github.com/lovlyDev/FormaCAD/actions/runs/41813913691).
This checkpoint predates the metadata-refresh addition above; it does not verify
that subsequent code. Workspace task filtering, remaining geometry consumers and folder UI
are still incomplete. This inspection route does not complete stage 4.

### Window-bound attachments

`backend::RepositoryRead` keeps one `ProjectRepository` together with its original
`RepositorySnapshot`. Native readers can take this bound value through queued
work and recheck the same saved state afterward. Managed callers explicitly
choose metadata capture or `capture_with_history`; no metadata-only managed view
can acquire an archive implicitly. For FCP1, `from_folder_view` adopts the exact
original host view and verifies it against the supplied opening before returning;
it never reopens a path or replaces the view with a fresh capture.

Window metadata and attachment readers use this same common bound route. Request
`close` revokes only that reader; the window still owns the shared source. Source
close revokes every retained reader. The attachment regression also checks refusal
of a foreign opening/view pair, request-close isolation, and rejection of history
after an external publication instead of adopting its newer saved state. These
bound-reader changes still need exact-head compilation and tests. Geometry IPC
consumers are the subsequent shared integration step.

Rust hosts can call `WindowSources::read_asset(windowLabel, sourceId, projectId,
expectedCommitId, name)`. It shares the expected-source capture used for metadata
reads, selects the file descriptor from that captured project and reads through
the same `FolderProject`. Callers provide no path, size or checksum authority.
The existing bounded CAS reader verifies the saved descriptor and bytes; the
original view is rechecked after data-URL encoding as well. There is no managed
blob fallback or app-index dependency. Unsupported platforms refuse the route.

The shared `read_project_file` IPC now accepts `sourceId` and `expectedCommitId`
together, using the actual invoking window. Calls without both fields retain
the managed route; a partial pair or invalid folder reference is refused without
fallback. `readProjectFile` and `readProjectSnapshot` accept the same optional
`FolderSourceRef` type. Browser calls with a folder reference refuse the route
before managed lookup or invoke. Source quota and unsupported-environment errors
have RU/EN translations. The real-folder regression uses two sources with
identical project IDs and filenames but different bytes, checks pure SQL bytes,
refuses a foreign commit/missing name/path traversal, then rejects an old commit
after an actual publication and rejects a closed source. These are attachment
byte tests; geometry acceptance is covered by the real worker tests below.
On `1c15f96ba607d2745c26889c889696b27a15ee42`,
[Storage comparison](https://github.com/lovlyDev/FormaCAD/actions/runs/41807057154)
passed: 98 folder tests, two window/asset tests, both strict Clippy modes and both
measurement steps. This does not validate the subsequently applied attachment IPC patch
or a desktop geometry flow.

### Window IPC

The approved `WindowSources` host state is managed independently of `AppState`.
`open_project_folder` opens the native folder picker for the invoking Tauri
window, then returns `{sourceId, commitId, project}` or `null` for cancellation.
Its scope is acquired before awaiting the picker; a late result after window
destruction cannot create a new opening. Native paths are not IPC arguments.

The existing `read_project_snapshot` accepts `{id}` for managed projects or
`{id, sourceId, expectedCommitId}` for this folder. Both additional fields are
required together. It refuses a foreign window/key, wrong project or commit,
and rechecks the original view after copying the response. There is no fallback
from a bad source key to a managed row. `close_project_source({sourceId})`
revokes the selected opening; window destruction and host Drop revoke every
associated source. Reusing a window label creates a fresh scope.

No visible folder button, geometry/model-write IPC or recent index is provided
by this integration. Before adding that UI, translate `PROJECT_SOURCE_LIMIT`
and `PROJECT_FOLDER_UNSUPPORTED` at its RU/EN display boundary and pass source
identity through every associated operation. Technical details remain separate.
Unsupported platforms refuse folder opening rather than using managed storage.
The new real-folder window regression and strict compilation still need results
on the applied revision; the earlier verified checkpoint below predates this IPC.

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
preservation and close/reopen refusal; see the exact-revision evidence below.

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
executed PASS; the verified revision is recorded below.
With `native-worker-client`, the API also exposes `SelectedStepFile`, opaque
`StepRead`, single-use `StepDestinationApproval` and `transfer_to_folder` using
the same `ModelRuntime`. This delegates to existing [STEP transfer](storage-step-transfer.md)
without old app SQL or recovered parametric history. Hash/read and approval are
blocking file operations; a UI host must schedule them away from its UI thread.
A new synthetic regression covers foreign opening and destination data added
after approval. The real STEP roundtrip now calls this public API. Exact-head
native-client verification is recorded below.
Production UI/index, actual USB/NAS and a genuine 1.1
fixture remain incomplete gates. This API does not change versions or release.

### Verified backend checkpoint

On `c147bf9ca2b9e6811ed945feaa203845c6827227`, all three workflows completed
successfully: [Quality](https://github.com/lovlyDev/FormaCAD/actions/runs/41744522132),
[Storage comparison](https://github.com/lovlyDev/FormaCAD/actions/runs/41744514043)
and [Storage native consumers](https://github.com/lovlyDev/FormaCAD/actions/runs/41744513890).
Downloaded native job `126302133894` and comparison job `126294056666` logs
contain the executed test results, rather than only test registration.

| Gate | Observed result |
| --- | --- |
| Non-native folder packet | 98 passed, 0 failed, 6 ignored |
| Strict Clippy | Passed without native client and with native-worker-client |
| FCP1 scoped reads and storage representation measurements | One executed PASS each |
| Native library packet | 529 passed, 0 failed, 18 ignored |
| Worker smoke packet | 11 passed, 0 failed |
| Seven host integration suites | 41 passed, 0 failed |
| Independent folder CAD roundtrip | One explicit executed PASS |
| Public folder host API roundtrip | One explicit executed PASS |
| Selected STEP transfer roundtrip | One explicit executed PASS |

The independent roundtrip creates real width-40 then width-80 models, checks
geometry, undoes to width 40, closes, probes the saved model in a fresh process
with a nonce, reopens and redoes to width 80. The public API and selected STEP
routes also use the actual built worker, with fresh-process saved geometry
checks. The explicit ignored tests are invoked separately; the ordinary packet's
ignored counts are not claims that every ignored test was executed.

This proves the Linux CI backend paths on that SHA. It does not prove a Windows
desktop picker, integrated UI/IPC, physical USB/NAS behavior, or a genuine 1.1
migration fixture. The [window integration patch](../integration/storage-folder-window.en.md)
was only an unapplied review artifact at this checkpoint. Stage 4 remains open;
the complete seven-stage program is not accepted by these backend results.

## Original-folder measurements over existing commands

`measure_model_reference`, `measure_model_pair`, `section_model_reference` and
`snap_model_reference` accept optional `sourceId` and `expectedCommitId`. Obtain
the reference only from folder selection in the current window. Both fields absent
preserves managed behavior; incomplete pairs fail. The host checks original window,
project and commit, then passes the original RepositoryRead into CAD. Source errors
cannot fall back to a managed project sharing the ID. taskScopeId groups tasks and
is not accepted as authority for choosing a source.

Queued name/message changes remain compatible only after recapturing through that
same reader and comparing original geometry bindings. Close/reopen, folder
replacement or geometry changes invalidate late results. STEP reads verify backend
membership, size and hash before moving bytes into the existing native stager.
Measurements do not save models, history or SourceRef.

Low-level frontend APIs accept an optional FolderSourceRef. Workspace controls must
still pass it and invalidate selection/request state when switching sources; this
does not expose a completed folder feature. Without a native client, folder requests
return the translated PROJECT_FOLDER_UNSUPPORTED error. Reader/staging passed
[native CI at 64bde10](https://github.com/lovlyDev/FormaCAD/actions/runs/41908269630),
including both executed STEP/FCP1 regressions. IPC at `5cb38ef` also passed
[native checks](https://github.com/lovlyDev/FormaCAD/actions/runs/42042470156);
the GUI route remains a separate gate. See the [IPC contract](../integration/storage-measurement-source-ipc.en.md).

## Original-folder preview

`preview_model` and `cancel_model_preview` accept the same optional
`sourceId`/`expectedCommitId` pair. The host verifies actual window, project and
commit. Requests without the pair retain managed behavior; incomplete or stale
pairs fail. Source, program hash and verified GLB remain bound to one request.
Previews publish no revisions, consume no grants and write no model/history.

One common host retains the original RepositoryRead, verifies imported STEP through
the backend, uses the existing native stager/worker and rechecks after the worker
and after the final task-map cleanup await. Metadata/geometry changes, close/reopen
or source replacement invalidate results. FCP1 task/cancel groups use opening UUIDs;
equal project IDs remain separate. The global CAD semaphore and individual
cancellation tokens remain.

Frontend preview/cancel APIs accept an optional FolderSourceRef. Cancellation
requires a valid reference to the current commit; stale references cannot rebind
to another opening. Browser folder requests reuse translated PROJECT_FOLDER_UNSUPPORTED.
Worker inputs live only in app-level `.transient/previews`; Workspace removes them
on completion.

At `4b96158`, [native CI](https://github.com/lovlyDev/FormaCAD/actions/runs/42050145085)
executed all three preview regressions: genuine imported STEP, late close during
worker/cleanup await and equal-ID folders with independent cancellation. Their
separate invocation returned exactly three PASS; the full library packet passed
540 tests (18 ignored). See the [preview contract](../integration/storage-preview-source.en.md).
Workspace now retains an opening source binding and uses it for scene and primary
callback invalidation. Visible folder controls and the complete writer route remain
unwired; this integration does not complete stage 4.

## Renderer opening binding

Workspace publishes Project alongside a process-local `ProjectSourceBinding`.
Explicit managed/folder activation starts a new request family; metadata refresh
preserves it. The renderer UUID only rejects late callbacks; it never replaces host
source/window/commit validation or writer permission. Binding and source tokens are
not persisted in preferences or project data.

Source switches reset topology/body selection, immediately hide retired geometry
and dispose late objects. CommittedScene includes opening identity alongside its
revision/asset seals. Folder modelLoader reads host bytes even with an inline
candidate and rechecks the same source/commit after decoding. Ready geometry survives
metadata-only refresh; pending IO receives the updated commit reference. Loading
never substitutes an equal-ID managed project.

Managed save, navigation, revision events and primary CAD callbacks check their
original opening; an old reply cannot replace a folder view. Folder read references
cannot invoke managed persistence or acquire managed writer access. Access status
also includes the opening, preventing a cached write mode during same-ID reacquire.
PROJECT_SOURCE_WRITE_NOT_AUTHORIZED is translated in both RU/EN catalogs.

Root-window project-scoped folder drafts stay in RAM, without managed preference
reads/writes. Their family includes opening and preference key, so moving to another
revision key rejects an old setter. This is renderer infrastructure, not an available
folder writer UI. Editor/AI preview and remaining measurement/history/import/export
consumers use separate agreed patches. See the [workspace state contract](../integration/storage-workspace-source-state.en.md)
for exact scope and verification.

## Window operation with its original view

`WindowSources.capture_operation(window, sourceId, projectId, expectedCommit)`
returns a Rust-only `FolderOperation` keeping the original `Arc<FolderProject>`
and its `ReadView` together. The window's own registry validates the selector;
another source with equal project IDs cannot replace it. After significant awaits,
the host rechecks the original object; window/source close revokes it. `into_read`
adopts that capture into the common RepositoryRead. Existing window readers use
this route without additional history archive reads.

Capture grants neither a writer lease nor user permission. Only a trusted host
**after its explicit permission decision for the operation** calls `approve_write`;
a renderer selector or boolean cannot substitute for that decision. The object is
not serialized or persisted and exposes no independently replaceable source/view.
`commit`, `apply_model` and `apply_history` pass the original view and opaque
WriteApproval to the existing transactional backend. CAD retains the common
executor; `None` permits only its existing no-op route. A raw renderer Project is
not a valid host-prepared candidate. The existing publisher still acquires the
writer OS-lock before writing.

`save_copy` uses the original view without approving source writes and creates an
independent identity in a separately host-selected destination. No new IPC or
visible button is added. Permission UI and writer IPC adoption still need shared
integration and agreement.

Four `window_sources::operation::tests` cases exercise actual SQL/CAS: approval
cannot transfer between equal-ID folders; close/reused window labels revoke the
old object; delayed approval refuses a newer committed state; read-only copying
works while another writer owns the source OS-lock, preserving its SQLite bytes.
Metadata changes here are not CAD geometry evidence. The new suite is verified
in current-commit CI; test presence and rustfmt alone are not PASS.

## Editor preview and AI-review

After agreement for eight frontend files, `useModelPreview` receives the original
opening binding. ProgramDialog passes it to preview/cancel and keeps folder drafts
in RAM through source preferences. Opening changes immediately hide/dispose retired
previews; cancellation targets only the original source/task group. Late replies
and errors cannot appear in a new equal-ID opening. Explicit missing or mismatched
bindings never become managed IPC calls. Metadata commits restart strict editor
preview; text edits in the same source retain serialized scheduling and debounce.

AI-review captures its binding before confirmation awaits. Candidate preview and a
separate saved GLB copy read from that source; the workspace scene is never reparented
into review. Opening or commit changes block acceptance, including metadata-only
commits: recapture cannot grant fresh authority to an old review. A new review is
required. Baseline caching also includes opening/commit and disposes late objects.
The agreed patch and separate commit regression fix are described in the
[frontend preview contract](../integration/storage-preview-renderer-source.en.md).

All three foundation CI runs passed at `6bd7924` before application; applied frontend
results are checked separately in Issue #31. This scope adds no visible folder
controls, writer UI or app installation. Other native consumers and the full user
read/write route still require completion of stage 4.

## Inspector, selected-body and pair measurements

App passes the original opening binding into SelectionModelProperties. Inspection
uses FolderSourceRef in existing inspect_model; its query cache includes opening,
source/task group/commit alongside project/revision. Cached or late properties from
an equal-ID previous opening cannot replace the current view. An explicit missing
or mismatched binding disables exact querying, retaining the mesh estimate without
managed fallback. Metadata commit changes restart strict inspection.

Selected-body and topology-pair measurements receive the binding with their original
geometry context. Identity includes opening/source/task group, revision/body/query,
saved program and primary STEP SHA/size; pairs also include scene token and captured
authored references. Native APIs use the capture's own source; an explicit foreign
source/commit/group override is refused. Opening changes invalidate reports, pair
slots and retired capture/clear callbacks.

Same-opening metadata-only refresh preserves compatible reports and selections.
The running request retains its original capture; sessions refresh only the next
request's commit reference. This never recaptures a different source: the backend
independently checks the original RepositoryRead, authored bindings and actual STEP
assets. Unlike strict AI-preview review, compatible measurements need not reset
their result merely because a name or messages changed.

Folder disclosure preferences for these panels remain in RAM, without managed
preference IO or persisted process-local tokens. Direct legacy callers may omit
binding; production UI supplies it explicitly. See the [renderer measurement contract](../integration/storage-measurement-renderer-source.en.md)
for exact 16-file scope and checks. Face/manual sections, history/import/export
and writer/permission UI remain subsequent integrations; visible folder controls
are not added yet.
### Snap from the displayed source scene

Pointer and context-menu operations use the read context of the actual loaded
object. Captured before loading, it remains in a WeakMap and never enters GLB/JSON.
Requests pass the original FolderSourceRef. Replies after opening or geometry
changes, or newer cursor placement, cannot change cursor position or error state.
Compatible metadata refresh preserves the running request's original commit; the
next request captures the new commit. Retired menus cannot move another folder's
cursor. Host stale-snapshot refusals do not fall back to estimates. Ordinary worker
failures may return an explicitly non-exact estimate only in the original current
context. See the [snap contract](../integration/storage-snap-renderer-source.en.md).

### One-use host decision for the original command

The Rust `window_sources/approval` module retains the original `FolderOperation`
and typed Model/History command before the user decision. `request_write` checks
window, source ID, project ID and expected commit. `resolve_write` accepts a decision
only for that window; `consume_write` requires the same original identity.
`ApprovedCommand.execute` executes the retained command without accepting a replacement
payload after approval. The common executor validates CAD programs; the host must
validate input sizes before creating a request from IPC.

Requests remain in RAM for at most five minutes, with at most 128 pending entries.
Restart or source/window closure never restores permission. Denial, repeated
resolution, replay, another same-ID folder and saved-state changes cannot authorize
a write. ReadSession and audit/settings are not permission authority. The existing
publisher still checks history and obtains the actual OS writer lease. Requesting
or deciding alone does not write project data.

The global registry mutex never spans an await. A per-request mutex serializes
resolution and consumption. Nine regressions use genuine FCP1/SQLite folders for
original command retention, read-only decisions, foreign selectors, denial/expiry/
replay, close/reopen, changed commits, concurrent consumers, independent requests
and registry capacity. CI runs these alongside existing window-source tests.
Until CI succeeds the new Rust checks are not claimed as locally executed: this
machine lacks a complete Windows SDK.

`request_prepared_write` passes the original Project to pure host command preparation
after validating the selected identity. Existing permission/command IPC can therefore
reuse common validators without capturing the source a second time. Preparation
failure creates no pending request. A wrong commit does not even invoke preparation.

The owned Storage comparison/native workflows preserve a running job on the next
push and queue the next run (`cancel-in-progress: false`). Checks are not reduced;
an unexecuted intermediate SHA is never claimed as verified. The shared Quality
workflow is unchanged. Results always belong to the exact run SHA.

A separate explicit native CI test executes retained CAD commands for 40→80 mm,
then Undo/Redo through the same one-use route and inspects actual STEP files with
the worker to check volume and bounds. It requires an explicitly built worker and
runs with `--ignored --exact`; a normal skipped/ignored run is not its validation.

This is a host foundation, without a new user action. Existing shared
`request_permission`/`resolve_permission` and model/history IPC are not connected
yet; their integration requires a separate agreed patch. No parallel IPC, managed
project rows or serialized grants are added. Folder controls remain hidden until
the complete read/write flow is ready.

### Manual section of the original STEP source

Existing `section_model` accepts an optional sourceId/expectedCommitId pair. The
host checks the original window and passes the selected folder's RepositoryRead to
the same CAD worker; invalid or incomplete pairs never fall back to managed storage.
Reading raw STEP does not require authored program. The original snapshot, revision
and sealed bytes are checked strictly before and after the worker, including workspace
cleanup. Metadata-only commits also invalidate manual sections. Verified bytes move
into the common stager without a second complete staging copy.

The six files in the [exact contract](../integration/storage-manual-section-source.en.md)
were explicitly agreed and applied. Production build and rustfmt passed locally.
A new genuine FCP1/STEP test checks the 120 mm perimeter with a corrupted managed
twin and refusal after source closure; its explicit native CI PASS is confirmed on fcd7ab1 (native42226643167).
The frontend API and production caller of both section modes are connected after
separate agreement for the nine files in the
[renderer contract](../integration/storage-section-renderer-source.en.md).

Manual sections retain the original commit, hide late result/error after opening
changes and restart after metadata commits. Selected-face sections retain compatible
geometry results on metadata refresh; the next request uses the fresh reference.
No running request is retargeted to another folder. Both modes keep the existing
single-unfinished-native-call sequencing.

The actual loaded object's context comes from its WeakMap entry. Retained geometry
keeps its local GPU plane only within the same opening; exact curves wait for current
displayed geometry. An unregistered folder object never receives managed fallback.
Folder panel preferences stay in RAM and reset on a new opening without reading or
writing managed preferences. Complete folder writer UI is still unfinished and
folder controls remain hidden.
