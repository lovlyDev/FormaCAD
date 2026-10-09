# Project session access

[Documentation](index.md) · [Русский](../ru/project-access.md) · [Folder storage](project-storage-v2.md)

Opening an indexed project calls `acquire_project_access(projectId)`. An OS lock held by the application grants `write`; another process receives `read_only`, including the owner PID when available. `project_access_status` returns `write`, `read_only` or `closed`. Closing calls `release_project_access`; an active project task returns `PROJECT_ACCESS_BUSY`. The OS releases ownership after process termination. Retry acquisition explicitly after the other owner closes; read-only sessions never silently promote themselves.

`AppState.project_access` owns a per-instance registry, separate from short transaction locks. Persistent lock files live in the managed root's `.project-access/<project UUID>.lock` and are never removed or force-stolen. PID, acquisition time and random session ID describe the owner; OS ownership is authoritative. Listing and reading ordinary projects do not acquire sessions. A save of an unopened project uses a scoped lease, released when the operation finishes, so dashboard metadata saves do not lock every project. Nested operations share an `Arc<File>` guard. CAD application, STEP conversion and AI execution hold guards for the whole task. Saving, deletion and mutation permissions enforce ownership in Rust before changes; disabling confirmation does not bypass read-only access.

Read-only sessions can inspect committed data and export mesh/bundles. STEP exports read immutable artifacts or generate selected geometry in a separate `.transient` workspace, then save to the user's destination; project `cache` and `output` are not changed. A reader may inspect a verified journaled generation while another owner completes folder publication, but cannot publish that generation or clear the pending journal. Temporary export folders are removed after completion; an abrupt crash may leave unreferenced transient files.

`copy_project_for_editing(projectId)` captures committed project data and logical history, hydrates immutable attachments and saves a new project UUID. Revision IDs and history are retained. The source is unchanged, including when it is read-only or still uses legacy database-backed storage. It does not transfer machine settings, permissions or CLI credentials. The copy receives ownership when opened; copy creation alone does not retain a lease.

This implementation coordinates processes using the same managed project root. It does not implement external-folder in-place editing, distributed/cloud lock recovery, heartbeat, force-unlocking or a NAS consistency guarantee. The UI must acquire before enabling editors and release on navigation; backend checks remain authoritative.

Isolated tests in `src/project_access/tests.rs` cover two registries, actual second-process contention and forced process exit, read-only save rejection before index/artifact changes, permission enforcement before audit changes, scoped/nested lease release, and copying without modifying or acquiring the original. They use temporary directories and test SQLite, never user AppData.

Windows verification: six primary tests passed in `.local/native-access-2.0-final.log`. A seventh subprocess-only helper is ignored in ordinary execution, but the actual second-process test launches it and terminates the owner. PID is read from a separate bounded `.owner.json` because Windows prevents reading exclusively locked byte ranges.

## Interface and switching

A read-only workspace explains the restriction and offers Retry editing access and Save an editable copy. Viewing, selection and measurements remain available; model changes, undo/redo and AI requests are blocked. A copy gets a new UUID and portable history without changing its source. Switching projects remains available. `features/project-access` serializes open/close IPC: a 250 ms release delay prevents losing the lease during StrictMode remounts, and reacquiring waits for an already-started release. Release errors are reported. Backend ownership checks remain the final protection.

## Pure reads for copying and exporting

Copying a current managed project and exporting `.cadpack` use a [shared project/history snapshot](project-snapshots.md) from one SQL snapshot, with folder-generation checks and a recheck after attachment reads before publication. These operations do not implicitly recover the source. Reopen a project that requires recovery, then retry. Measurements retain their compatible error codes. Saving a new copy inserts a new project into shared SQLite; the source project remains unchanged, but the complete database file need not remain byte-identical. This changes the managed backend, not external-folder in-place opening or editable-history conversion of a genuine 1.1 project.
