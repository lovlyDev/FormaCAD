# Applying a model through a checked host transaction

[Documentation](index.md) · [Русский](../ru/model-apply-transactions.md) · [Modeling](modeling.md) · [History](project-history.md) · [AI candidate review](ai-candidate-review.md)

The model editor's Build action and acceptance of an AI candidate use one host application route. A command batch uses the same route after the batch is staged against its expected revision. Geometry construction, a preview and a committed project are separate results: only checked output can become a new revision.

## User actions and failures

The editor captures its project ID and current head before confirmation. Switching projects or moving the head invalidates that approval; it never retargets a draft. Errors and assistant messages from a delayed request are limited to that captured project. An unchanged result preserves selection, the loaded GLB instance, camera and persisted editor draft.

While a committed GLB is loading, the last valid geometry remains visible for the same project with an explicit loading state. CAD selection, measurement and model-history actions are blocked against that obsolete display. The old section clipping plane remains stable, but fresh-revision native section reports and curves wait until the new mesh is displayed. A failed load offers **Retry preview loading** and retains the previous valid model; it never substitutes a generated parameter primitive. Switching projects hides the prior project's mesh immediately; an empty history head clears it. Shared source geometry and materials are released once, after renderer appearance and section cleanups restore borrowed originals.

The renderer listens for `project://revision-created` identities and fetches authoritative project snapshots. Repeated notifications for an already adopted head do not reload geometry or clear selection. A fetch delayed across a project switch or a newer invoke result cannot overwrite the active workspace. Unmount removes the listener. If geometry committed successfully but optional assistant-message persistence fails, the error explicitly describes the message failure; the saved model stays adopted.

Edit the draft and build it, or review an AI candidate and accept it. When confirmation is required, approve the existing permission request. A successful build adds one revision with sealed STEP and GLB, and one model-history edit. Undo and redo retain the existing immutable-history behavior. An unchanged saved source without a review plan returns the current project without another edit or permission consumption.

If the kernel rejects an edge reference or radius, an AI expectation fails, the request becomes stale or the task is cancelled before commit, the saved model remains available. Correct the draft or regenerate it from the current revision before retrying. The application does not choose a nearby edge to repair an invalid reference. Cancellation after the storage transaction is durable does not erase that revision.

## Technical route and module responsibilities

During retained-preview loading or failure, the overlay identifies the displayed revision and the properties panel reads that same immutable revision, rather than attaching new-head measurements to the old mesh. Event snapshot guards use immutable project object identity: a newer same-head message/thumbnail save also wins over an older pending fetch. An unrelated event replaces only its own project cache entry.

The [IPC facade](../../apps/desktop/src-tauri/src/modeling.rs) handles Tauri arguments and notifications. [Host apply](../../apps/desktop/src-tauri/src/modeling/apply.rs) accepts `&AppState` and a model request, without `AppHandle` or a UI runtime. [Command staging](../../apps/desktop/src-tauri/src/modeling/commands.rs) checks the expected revision, applies at most 256 typed commands on a copy and supplies the intended new revision identity to that same host route.

Host apply holds the project's write-access guard, validates input size and structure, compares the expected head, validates any review plan and handles the unchanged-source case. It consumes the existing `modify_project` permission, registers the project task and acquires the shared CAD permit. After queue waiting it rechecks the captured head and file list before doing expensive work. It never replaces a stale captured document with a newer document silently.

[Build](../../apps/desktop/src-tauri/src/modeling/build.rs) stages verified imported STEP inputs or a sealed legacy base, then invokes the existing isolated native worker or compatibility runner. Worker request identity, output limits and checksums remain authoritative. AI review expectations are evaluated against the fresh verified worker response, not provider claims or preview metrics. The build returns owned STEP/GLB bytes after its scoped workspace has been removed.

After releasing the heavy CAD permit, host apply checks cancellation, acquires the write mutex and repeats cancellation and head/file checks. [Commit](../../apps/desktop/src-tauri/src/modeling/commit.rs) constructs the next project on a copy, binds the stored document's `revisionId` to the saved revision, normalizes the artifacts and calls the existing project persistence transaction once. Persistence records model history and storage recovery data through its established SQL transaction. The IPC facade emits the revision-created event only for a committed outcome from this route; unchanged or rejected apply outcomes do not emit it.

The Rust-only explicit-worker function supports integration tests. It passes through the same access, permissions, task, queue, review and commit checks. A worker executable path is not part of the IPC request or CAD document.

## Storage, permissions and cleanup

New generated build files live in an owned `.transient/model-builds/{project-id}-{unique-id}` directory outside durable project directories. [Workspace ownership](../../apps/desktop/src-tauri/src/modeling/workspace.rs) checks the generated path and removes only that directory on ordinary completion, error or cooperative cancellation. Old project cache files, previous exports and released installers are retained. Saved artifacts remain content-addressed and immutable; the project folder and SQLite history are handled by [project storage](project-storage-v2.md).

Task cleanup removes only the registration whose cancellation token belongs to the current job. A separate busy job is preserved. This route does not promise cleanup after arbitrary future abortion or panic. Temporary deletion failures are logged; they do not authorize deleting other directories.

Permissions retain their existing policy. A missing, expired or other-project approval prevents execution. A consumed approval is single-use even if geometry fails later. Automatic mode can append a permission audit record for a subsequently rejected model. Those records and session-lock metadata are not model commits and are not rolled back by model failure.

Permission consumption precedes the busy-task check: a request rejected because another project task is running can consume its approval. The unchanged-source path returns before either step and preserves another task's registration. Retry a rejected edit through the normal permission flow.

The no-publication guarantee here concerns kernel, review, cancellation and stale-result failures before persistence starts. Persistence can stage immutable files before SQL commit, and its separate recovery protocol governs storage failures. These host tests do not prove rollback of every I/O failure or remove a durable SQL commit after a publication problem. See [recovery limitations](project-storage-v2.md).

After SQL commits, folder publication and compatibility mirrors (`project.json`, `workspace/model.py` and parameters) are best effort. A guarded-path or mirror failure is logged without turning the committed model into a failed apply. A failed folder publication retains the pending recovery journal; opening the project retries recovery when the required generation files are available. Guard checks remain mandatory, including when a directory is replaced by a link. Metadata-only saves do not repeat the revision-created event; history head movement retains that notification. These storage tests exercise the event predicate, not delivery inside an installed WebView.

## Compatibility and verification

The renderer regression in [model-apply-outcomes.spec.ts](../../apps/desktop/e2e/model-apply-outcomes.spec.ts) mounts the actual App, router and workspace with test-only IPC and real isolated-worker STEP/GLB fixtures. It checks delayed and failed preview replacement, GPU section retention, geometry identity, no-op, persisted drafts, rejected kernel references, captured approvals and delayed agent errors. Chromium/mock IPC verifies frontend lifecycle, not native transaction or installed WebView event delivery; the independent host suite below verifies storage and CAD execution. The section report in this renderer harness is explicitly mocked; its purpose is guarding evaluation timing and GPU clipping, not validating section geometry.

Native CAD IR v2 runs without Python. Typed legacy CAD JSON v1 keeps its existing compiler and reaches the legacy runner; it is not deserialized as native schema v2 while staging assets. Plain CadQuery retains its sealed-base behavior. Legacy execution still requires the configured local Python/CadQuery environment; compile and routing tests do not prove that environment is installed or available offline.

The [integration suite](../../apps/desktop/src-tauri/tests/model_apply_transaction.rs) uses a real isolated worker and temporary AppState/SQLite. It compares saved project payload, head, files, history events, manifest and artifact hashes. Cases cover wrong reference owner/path and oversized fillet radius reaching the kernel, a valid referenced fillet, fresh review expectation failure and acceptance, sealed commit/no-op/undo/redo, command revision identity, cancellation in the queue and after worker output, a changed head while waiting, imported-input corruption, single-use permissions, busy-task preservation and read-only rejection.

Compatibility units run with native features and the non-native compilation route. These checks do not replace installed WebView event tests, authenticated AI CLI acceptance, hard-abort safety, storage fault injection or three-platform acceptance. They do not establish general topology identity beyond the [bounded native references](topology-references.md).

The final-CAS regression deliberately holds the write mutex until a real build releases its CAD permit, publishes a valid independent winner through internal persistence, and verifies that the built candidate cannot append another revision or artifacts. This exercises the final host boundary; it does not claim that normal UI modeling can bypass the busy-task guard to race another approved model task.
